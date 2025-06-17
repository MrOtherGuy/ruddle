#![deny(warnings)]
use super::{resource::TryParseStringValue};
use crate::settings::ServerConfigError;
use crate::support::cryptea;

#[derive(Debug,Clone)]
pub enum CredentialsMode{
    Plain,
    Encoded
}

#[derive(Debug,Clone)]
pub struct ResourceCredentials{
    key: Vec<u8>,
    pub header: String,
    pub mode: CredentialsMode
}

impl ResourceCredentials{
    pub fn derive_key(&self,key: &str) -> Result<String,ServerConfigError>{
        match cryptea::decode(&self.key,key){
            Ok(s) => Ok(s),
            Err(e) => {
                println!("{}",e);
                Err(ServerConfigError::DecodeError)
            }
        }
    }
    pub fn key(&self) -> &Vec<u8>{
        &self.key
    }
    pub fn try_parse(table: &config::Map<String, config::Value>) -> Result<Self,ServerConfigError>{
        let resource_key = match table.try_parse_string("key_value"){
            Ok(k) => {
                let s : Option<Vec<u8>> = match k.as_bytes().try_into(){
                    Ok(slice) => Some(slice),
                    Err(_) => None
                };
                s
            },
            Err(_) => None
        };
        if resource_key.is_none(){
            return Err(ServerConfigError::NotAvailable)
        }
        let resource_header = match table.try_parse_string("key_header"){
            Ok(k) => {
                match k.len() > 4 && k.len() < 50 { // arbitrary restriction for header length
                  true => Some(k),
                  false => return Err(ServerConfigError::InvalidValue)
                }
            },
            Err(_) => None
        };
        let key_mode = match table.try_parse_string("key_mode"){
            Ok(k) => {
                match k.as_str(){
                    "plain" => CredentialsMode::Plain,
                    _ => CredentialsMode::Encoded
                }
            },
            Err(_) => CredentialsMode::Encoded
        };
        match resource_header {
            Some(header) => Ok(ResourceCredentials{
                key: resource_key.unwrap(),
                header: header,
                mode: key_mode
            }),
            None => Err(ServerConfigError::NotAvailable)
        }
        
    }
    
}