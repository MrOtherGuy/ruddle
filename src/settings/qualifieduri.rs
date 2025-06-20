#![deny(warnings)]
use crate::settings::ServerConfigError;
use std::collections::HashMap;
use http::uri::Builder;

#[derive(Debug)]
pub struct QualifiedUri{
    inner: hyper::Uri,
    query: QueryParams
}
#[derive(Debug)]
pub struct QueryParams{
    pub map : HashMap<String,Option<String>>
}

impl QueryParams{
    pub fn from_str(input: &str) -> Self{
        let mut map : HashMap<String,Option<String>> = HashMap::new();
        for item in input.split_terminator("&"){
            match item.split_once("="){
                Some((key,val)) => map.insert(key.to_owned(),Some(val.to_owned())),
                None => map.insert(item.to_owned(),None)
            };
        }
        QueryParams{map:map}
    }
    pub fn stringify(&self) -> Option<String>{
        if self.map.len() == 0{
            return None
        }
        let mut parts : Vec<String> = vec![];
        for (key,val) in self.map.iter(){
            let mut string = String::new();
            string.push_str(key);
            match val{
                Some(v) => {
                    string.push_str("=");
                    string.push_str(v);
                },
                None => ()
            }
            parts.push(string);
        }
        Some(parts.join("&"))
    }
    pub fn extend_with(&self, mut with: QueryParams) -> String{
        with.map.extend(self.map.iter().map(|(key,val)| (key.clone(),val.clone())));
        with.stringify().unwrap()
    }
}

impl QualifiedUri{
    pub fn try_build(input: String, disallowed_port: u16) -> Result<QualifiedUri,ServerConfigError>{
        let uri : hyper::Uri = match input.parse(){
            Ok(s) => s,
            Err(_) => { println!("Invalid uri: {}",input); return Err(ServerConfigError::InvalidURI) }
        };
        let scheme = match uri.scheme(){
            Some(s) => s,
            _ => return Err(ServerConfigError::InvalidURI)
        };
        let authority = match uri.authority(){
            Some(a) => a,
            _ => return Err(ServerConfigError::InvalidURI)
        };
        let default_port : u16 = match uri.scheme().unwrap().as_str(){
            "https" => 443,
            "http" => 80,
            _ => panic!("This shouldn't happen")
        };
        let host = uri.host().unwrap();
        if uri.port_u16().unwrap_or(default_port) == disallowed_port && (host == "localhost" || host == "127.0.0.1"){
            return Err(ServerConfigError::InvalidURI)
        }
        let constructed_uri = Builder::new()
            .scheme(scheme.as_str())
            .authority(authority.as_str())
            .path_and_query(uri.path())
            .build();
        match constructed_uri{
            Ok(c) => Ok(QualifiedUri{
                inner: c,
                query: QueryParams::from_str(uri.query().unwrap_or(""))
            }),
            Err(_) => Err(ServerConfigError::InvalidURI)
        }
        
    }
    pub fn uri(&self) -> hyper::Uri{
        if let Some(s) = self.query.stringify(){
            let mut constructed = self.inner.to_string();
            constructed.push_str("?");
            constructed.push_str(s.as_str());
            let uri : hyper::Uri = constructed.parse().unwrap();
            return uri
        }
        self.inner.clone()
    }
    pub fn composed(&self, params: QueryParams) -> Result<hyper::Uri,ServerConfigError>{
        let mut constructed = self.inner.to_string();
        constructed.push_str("?");
        constructed.push_str(self.query.extend_with(params).as_str());
        let uri : Result<hyper::Uri,_> = match constructed.parse(){
            Ok(uri) => Ok(uri),
            Err(_) => Err(ServerConfigError::InvalidURI)
        };
        uri
    }
}