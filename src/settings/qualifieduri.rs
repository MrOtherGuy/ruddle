#![deny(warnings)]
use crate::settings::ServerConfigError;

#[derive(Debug)]
pub struct QualifiedUri{
    inner: hyper::Uri
}

impl QualifiedUri{
    pub fn try_build(input: String, disallowed_port: u16) -> Result<QualifiedUri,ServerConfigError>{
        let uri : hyper::Uri = match input.parse(){
            Ok(s) => s,
            Err(_) => { println!("Invalid uri: {}",input); return Err(ServerConfigError::InvalidURI) }
        };
        match uri.scheme(){
            Some(s) => match s.as_str() {
                "http" | "https" => (),
                _ => return Err(ServerConfigError::InvalidURI)
            }
            _ => return Err(ServerConfigError::InvalidURI)
        }
        match uri.authority(){
            Some(_) => (),
            _ => return Err(ServerConfigError::InvalidURI)
        }
        let default_port : u16 = match uri.scheme().unwrap().as_str(){
            "https" => 443,
            "http" => 80,
            _ => panic!("How exiting, this shouldn't be able to happen")
        };
        let host = match uri.host(){
            Some(host) => host,
            None => return Err(ServerConfigError::InvalidURI)
        };
        if uri.port_u16().unwrap_or(default_port) == disallowed_port && (host == "localhost" || host == "127.0.0.1"){
            return Err(ServerConfigError::InvalidURI)
        }
        Ok(QualifiedUri{
            inner: uri
        })
    }
    pub fn uri(&self) -> &hyper::Uri{
        &self.inner
    }
}