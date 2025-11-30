#![deny(warnings)]

use serialport::{SerialPortType,ClearBuffer};

pub enum SerialPortError{
    NotAvailable,
    SerializationFailed,
    DeserializationFailed,
    NotWritable
}

impl std::fmt::Display for SerialPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            SerialPortError::NotAvailable => write!(f, "Serial ports could not be enumerted"),
            SerialPortError::SerializationFailed => write!(f, "Serialization failed"),
            SerialPortError::DeserializationFailed => write!(f, "Deserialization failed"),
            SerialPortError::NotWritable => write!(f, "Couldn't write to port")
        }
    }
}

#[derive(serde::Serialize,Debug)]
pub struct JSONPortList{
    ports: Vec<JSONPortInfo>
}

impl JSONPortList{
    pub fn from_ports(input: Vec<serialport::SerialPortInfo>) -> Self{
        let ports = input.into_iter().map(|x| JSONPortInfo::from_serialport(x)).collect();
        JSONPortList{ ports }
    }
}

#[derive(serde::Serialize,Debug)]
pub struct JSONPortInfo{
    name: String,
    #[serde(rename = "type")]
    port_type: String,
    #[serde(rename = "portInfo")]
    port_info: Option<UsbPortIdentifier>
}
#[derive(serde::Serialize,serde::Deserialize,Debug)]
struct UsbPortIdentifier{
    #[serde(rename = "USBProduct")]
    product: u16,
    #[serde(rename = "USBVendor")]
    vendor: u16
}
#[derive(serde::Deserialize,Debug)]
enum SerialPortTarget{
    PortName(String),
    PortQuery(UsbPortIdentifier)
}

pub enum BaudRateError{
    InvalidRate
}
impl std::fmt::Display for BaudRateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            BaudRateError::InvalidRate => write!(f, "Baud rate not supported")
        }
    }
}

#[derive(serde::Deserialize,Debug,Clone)]
#[repr(u32)]
#[serde(try_from = "u32")]
enum BaudRate{
    R75 = 75,
    R110 = 110,
    R300 = 300,
    R1200 = 1200,
    R2400 = 2400,
    R4800 = 4800,
    R9600 = 9600,
    R19200 = 19200,
    R38400 = 38400,
    R57600 = 57600,
    R115200 = 115200
}

impl BaudRate{
    fn as_u32(&self) -> u32{
        self.clone() as u32
    }
}

impl Default for BaudRate{
    fn default() -> Self{
        BaudRate::R9600
    }
}

impl TryFrom<u32> for BaudRate{
    type Error = BaudRateError;
    fn try_from(input: u32) -> Result<Self,Self::Error>{
        let br = match input{
            75 => BaudRate::R75,
            110 => BaudRate::R110,
            300 => BaudRate::R300,
            1200 => BaudRate::R1200,
            2400 => BaudRate::R2400,
            4800 => BaudRate::R4800,
            9600 => BaudRate::R9600,
            19200 => BaudRate::R19200,
            38400 => BaudRate::R38400,
            57600 => BaudRate::R57600,
            115200 => BaudRate::R115200,
            _ => return Err(BaudRateError::InvalidRate)
        };
        Ok(br)
    }
}

#[derive(serde::Deserialize,Debug)]
pub struct SerialPortData{
    target: SerialPortTarget,
    #[serde(rename = "baudRate")]
    #[serde(default)]
    baud_rate: BaudRate,
    data: String
}

impl SerialPortData{
    fn select(&self) -> Result<serialport::SerialPortBuilder,SerialPortError>{
        match &self.target{
            SerialPortTarget::PortName(name) => Ok(serialport::new(name,self.baud_rate.as_u32())),
            SerialPortTarget::PortQuery(query) => {
                let mut ports = match serialport::available_ports() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{}",e);
                        return Err(SerialPortError::NotAvailable)
                    }
                };
                ports.retain(|port| match &port.port_type{
                    SerialPortType::UsbPort(info) => info.vid == query.vendor && info.pid == query.product,
                    _ => false
                });
                let port = match &ports.len(){
                    0 => return Err(SerialPortError::NotAvailable),
                    1 => ports.pop(),
                    _ => {
                        eprintln!("Multiple potential ports found, picking last");
                        ports.pop()
                    }
                };
                Ok(serialport::new(&port.unwrap().port_name,self.baud_rate.as_u32()))
            }
        }
    }
    pub fn write(&self) -> Result<(),SerialPortError>{
        let port = self.select()?;
        match port.open(){
            Ok(mut open) => {
                match open.clear(ClearBuffer::All){
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}",e);
                        return Err(SerialPortError::NotWritable)
                    }
                }
                match open.write(self.data.as_bytes()){
                    Ok(_) => {
                        Ok(())
                    },
                    Err(e) => {
                        eprintln!("{:?}", e);
                        Err(SerialPortError::NotWritable)
                    }
                }
            },
            Err(e) => {
                eprintln!("{}",e);
                Err(SerialPortError::NotAvailable)
            }
        }
    }
    pub fn try_from_bytes(input: Vec<u8>) -> Result<Self,SerialPortError>{
        let info : SerialPortData = match serde_json::from_slice(input.as_slice()){
            Ok(info) => info,
            Err(e) => {
                eprintln!("{}",e);
                return Err(SerialPortError::DeserializationFailed)
            }
        };
        Ok(info)
    }
}

impl JSONPortInfo{
    pub fn from_serialport(input: serialport::SerialPortInfo) -> Self{
        use serialport::{SerialPortType};
        let name = input.port_name;
        let port_info = match input.port_type{
            SerialPortType::UsbPort(info) => ("USB".to_string(),Some(UsbPortIdentifier{product: info.pid, vendor: info.vid})),
            SerialPortType::PciPort => ("PciPort".to_string(),None),
            SerialPortType::BluetoothPort => ("Bluetooth".to_string(),None),
            SerialPortType::Unknown => ("Unknown".to_string(),None)
        };
        
        JSONPortInfo{ name, port_type: port_info.0, port_info: port_info.1 }
    }
}

pub fn enumerate_available_ports() -> Result<Vec<u8>,SerialPortError>{
    
    let ports = match serialport::available_ports() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}",e);
            return Err(SerialPortError::NotAvailable)
        }
    };
    let js_ports = JSONPortList::from_ports(ports);
    match serde_json::to_vec(&js_ports){
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            eprintln!("{}",e);
            Err(SerialPortError::SerializationFailed)
        }
    }
}
