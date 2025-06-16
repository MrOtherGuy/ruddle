#![deny(warnings)]
use base64::{Engine,engine::general_purpose};
use std::num::Wrapping;

pub type CrypTeaResult<T> = Result<T, CrypTeaError>;

impl std::fmt::Display for CrypTeaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            CrypTeaError::InvalidUTF8 => write!(f, "Invalid bytest for utf8!"),
            CrypTeaError::DecodeError => write!(f, "Could not decode bytes"),
            CrypTeaError::EncodeError => write!(f, "Cannot encode zero size str")
        }
    }
}
#[derive(Debug)]
pub enum CrypTeaError{
    InvalidUTF8,
    DecodeError,
    EncodeError
}
struct LongData{
    data: Vec<u32>
}

impl LongData{
    fn get_at(bytes: &[u8], idx: usize) -> u32{
        return (bytes[idx * 4] as u32)
        + ((bytes[idx * 4 + 1] as u32) << 8)
        + ((bytes[idx * 4 + 2] as u32) << 16)
        + ((bytes[idx * 4 + 3] as u32) << 24)
    }
    fn from_bytes(bytes: &[u8]) -> CrypTeaResult<LongData>{
        let min_size = (bytes.len() >> 2) + match bytes.len() % 4{ 0 => 0, _ => 1 } - 1;
        let mut buf : Vec<u32> = Vec::with_capacity(min_size+1);
        for n in 0..min_size{
            buf.push(LongData::get_at(bytes,n));
        }
        match bytes.len() % 4 {
            0 => buf.push(LongData::get_at(bytes,min_size)),
            1 => buf.push(bytes[min_size * 4] as u32),
            2 => buf.push(bytes[min_size * 4] as u32 + ((bytes[min_size * 4 + 1] as u32) << 8)),
            3 => buf.push(bytes[min_size * 4] as u32 + ((bytes[min_size * 4 + 1] as u32) << 8) + ((bytes[min_size * 4 + 2] as u32) << 16)),
            _ => panic!("This cannot happen")
        }
        Ok(LongData{
            data: buf
        })
    }
}

pub fn decode(input: &Vec<u8>,key: &str) -> Result<String,CrypTeaError>{

    let bytes = match general_purpose::STANDARD.decode(input){
        Ok(decoded) => decoded,
        Err(e) => {
            eprintln!("{}",e);
            return Err(CrypTeaError::DecodeError)
        }
    };

    let mut data = LongData::from_bytes(bytes.as_slice())?;

    let key = LongData::from_bytes(key.as_bytes())?;
    let data_len = data.data.len();

    let mut z : u64;
    let mut y : u64 = data.data[0].into();
    let mut e : usize;

    let delta : u64 = 0x9E3779B9;
    let q : u8 = 6 + (52 / data_len) as u8;

    let mut sum : u64 = (q as u64 * delta).into();

    while sum > 0{
        e = (sum >> 2) as usize & 3;
        for p in (1..=(data_len - 1)).rev(){
            z = data.data[p - 1].into();

            let sum1: u64 = ((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64;
            let sum2: u64 = (sum ^ y) as u64 + (key.data[p & 3 ^ e] as u64 ^ z);

            let r = Wrapping((sum1 ^ sum2) as u32);
            let w = Wrapping(data.data[p]);
            data.data[p] = (w - r).0;

            y = data.data[p].into();
        }
        z = data.data[data_len - 1] as u64;
        let r = Wrapping(((z >> 5 ^ y << 2) + (y >> 3 ^ z << 4) ^ (sum ^ y) + (key.data[e] as u64 ^ z)) as u32);
        let w = Wrapping(data.data[0]);
        data.data[0] = (w - r).0;
        y = data.data[0] as u64;
        sum = sum - delta
    }
    let mut out_vec : Vec<u8> = Vec::with_capacity(data.data.len() * 4);
    data.data.iter().for_each(|x| {
        out_vec.push((x & 0xff).try_into().unwrap());
        out_vec.push(((x & 0xff00) >> 8).try_into().unwrap());
        out_vec.push(((x & 0xff0000) >> 16).try_into().unwrap());
        out_vec.push((x >> 24).try_into().unwrap())
    });
    
    while out_vec.pop_if(|x: &mut u8| *x == 0).is_some(){};
    
    return match String::from_utf8(out_vec){
        Ok(s) => Ok(s),
        Err(e) => {
            eprintln!("{}",e);
            Err(CrypTeaError::InvalidUTF8)
        }
    }

}
pub fn encode_to_bytes(input: &str,key: &str) -> Result<Vec<u8>,CrypTeaError>{
    if input.len() == 0{
        return Err(CrypTeaError::EncodeError)
    }
    let mut data = LongData::from_bytes(input.as_bytes())?;
    
    let key = LongData::from_bytes(key.as_bytes())?;
    let data_len = data.data.len();
    let n = data_len - 1;
    let mut z : u64 = data.data[n].into();
    let mut y : u64;
    let mut e : usize;
    
    let delta : u64 = 0x9E3779B9;
    let mut q : u8 = 6 + (52 / data_len) as u8;
    
    let mut sum : u64 = 0;
    
    while q > 0{
        q = q-1;
        sum = sum + delta;
        e = (sum >> 2) as usize & 3;
        for p in 0..n{
            y = data.data[p + 1].into();

            let sum1: u64 = ((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64;
            let sum2: u64 = (sum ^ y) as u64 + (key.data[p & 3 ^ e] as u64 ^ z);

            let r = Wrapping((sum1 ^ sum2) as u32);
            let w = Wrapping(data.data[p]);
            
            let t = (w + r).0;
            data.data[p] = t;
            z = t.into();
        }
        y = data.data[0] as u64;
        let r = Wrapping(((z >> 5 ^ y << 2) + (y >> 3 ^ z << 4) ^ (sum ^ y) + (key.data[n & 3 ^ e] as u64 ^ z)) as u32);
        let w = Wrapping(data.data[n]);
        let t = (w + r).0;
        data.data[n] = t;
        z = t.into();
    }
    let mut out_vec : Vec<u8> = match input.len() % 4{
        0 => vec![0; input.len()],
        a => vec![0; input.len() + (4 - a)]
    };

    for (idx, x) in data.data.iter().enumerate(){
        out_vec[idx * 4] = (x & 0xff).try_into().unwrap();
        out_vec[idx * 4 + 1] = ((x & 0xff00) >> 8).try_into().unwrap();
        out_vec[idx * 4 + 2] = ((x & 0xff0000) >> 16).try_into().unwrap();
        out_vec[idx * 4 + 3] = (x >> 24).try_into().unwrap();
    };
    return Ok(out_vec)

}
pub fn encode_as_base64(input: &str, key: &str) -> Result<String,CrypTeaError>{
    match encode_to_bytes(input,key){
        Ok(bytes) => Ok(general_purpose::STANDARD.encode(bytes)),
        Err(e) => Err(e)
    }
}