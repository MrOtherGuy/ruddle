#![deny(warnings)]

use crate::settings::ServerConfigError;
use crate::models::{RemoteResultType,RemoteData};
use crate::schemers::{schemaloader::SchemaTree};
use std::path::PathBuf;
use super::qualifieduri::QualifiedUri;
use super::credentials::{CredentialsMode,ResourceCredentials};



pub struct RequestCredentials{
    pub key: String,
    pub value: String
}

#[derive(Debug,Clone)]
pub struct WriteTarget{
    path: PathBuf
}

impl WriteTarget{
    fn new(inpt: String) -> Self{
        WriteTarget{
            path: PathBuf::from(inpt)
        }
    }
    pub async fn write_file(&self,stream: &RemoteData) -> Result<(),std::io::Error>{
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&self.path).await?;
        file.write_all(stream.data_bytes()).await?;
        Ok(())
    }
}

pub trait TryParseStringValue{
    fn try_parse_string(&self,key : &str) -> Result<String,ServerConfigError>;
}

impl TryParseStringValue for config::Map<String, config::Value>{
    fn try_parse_string(&self,key : &str) -> Result<String,ServerConfigError>{
        match self.get(key){
            Some(value) => match value.clone().into_string(){
                Ok(k) => Ok(k),
                Err(_) => Err(ServerConfigError::InvalidValue)
            },
            None => Err(ServerConfigError::MissingKey)
        }
    }
}

#[derive(Debug)]
pub struct RemoteResource{
    pub uri: QualifiedUri,
    credentials: Option<ResourceCredentials>,
    pub target: Option<WriteTarget>,
    pub model: crate::models::RemoteResultType,
    cache: std::sync::OnceLock<RemoteData>,
    pub schema: Option<String>,
    pub no_cache: bool
}
#[allow(unused)]
impl RemoteResource{
    pub fn derive_key(&self, key: &str) -> Result<String,ServerConfigError>{
        match &self.credentials {
            Some(cred) => cred.derive_key(key),
            None => Err(ServerConfigError::NotAvailable)
        }   
    }
    pub fn request_credentials(&self, key: &str) -> Option<Result<RequestCredentials,ServerConfigError>>{
        match &self.credentials{
            Some(c) => match &c.mode{
                CredentialsMode::Plain => match String::from_utf8(c.key().clone()){
                    Ok(s) => Some(Ok(RequestCredentials{ key: c.header.clone(), value: s})),
                    Err(_) => Some(Err(ServerConfigError::DecodeError))
                },
                CredentialsMode::Encoded => match c.derive_key(key) {
                    Ok(key) => Some(Ok(RequestCredentials{ key: c.header.clone(), value: key })),
                    Err(_) => Some(Err(ServerConfigError::DecodeError))
                }
            },
            None => None
        }
    }
    pub fn get_cached(&self) -> Option<&RemoteData>{
        self.cache.get()
    }
    pub fn cache_result(&self,data: RemoteData) -> Result<RemoteData,ServerConfigError>{
        if self.no_cache {
            return Err(ServerConfigError::NotAvailable)
        }
        match self.cache.set(data.clone()){
            Ok(_) => Ok(data),
            Err(e) => {
                eprintln!("{:?}",e);
                Err(ServerConfigError::NotAvailable)
            }
        }
    }
    pub fn try_from_config(conf: &config::Value, disallowed_port: u16, tree: &Option<SchemaTree>) -> Result<RemoteResource,ServerConfigError>{
        try_into_remote(conf,disallowed_port,tree)
    }
}

fn try_into_remote(conf: &config::Value,disallowed_port: u16, tree: &Option<SchemaTree>) -> Result<RemoteResource,ServerConfigError>{
    match conf.clone().into_table(){
        Ok(table) => match table.try_parse_string("url"){
            Ok(url_string) => {
                let uri_conversion = QualifiedUri::try_build(url_string,disallowed_port);
                if uri_conversion.is_ok(){

                    let creds = match ResourceCredentials::try_parse(&table){
                        Ok(cred) => Some(cred),
                        Err(e) => {
                            println!("{}",e);
                            match e {
                                ServerConfigError::InvalidValue => return Err(ServerConfigError::MissingKey),
                                _ => None
                            }
                        }
                    };
                    
                    let write_target = match table.try_parse_string("file_target"){
                        Ok(k) => Some(WriteTarget::new(k)),
                        Err(_) => None
                    };
                    let data_model = match table.try_parse_string("model"){
                        Ok(k) => match RemoteResultType::from_str(&k){
                            Some(t) => t,
                            None => RemoteResultType::RemoteBytes
                        },
                        Err(_) => RemoteResultType::RemoteBytes
                    };
                    let no_cache = match table.get("no_cache"){
                        Some(k) => match k.clone().into_bool(){
                            Ok(b) => b,
                            Err(_) => return Err(ServerConfigError::InvalidValue)
                        }
                        None => false
                    };
                    let schema = match &tree{
                        Some(onetree) => match table.get("schema"){
                            Some(s) => match s.clone().into_string(){
                                Ok(t) => match onetree.contains_schema(&t){
                                    true => Some(t),
                                    false => return Err(ServerConfigError::UnsupportedSchema)
                                },
                                Err(_) => return Err(ServerConfigError::UnsupportedSchema)
                            },
                            None => None,
                        },
                        None => None
                    };
                    if !data_model.supports_schema() && schema.is_some(){
                        return Err(ServerConfigError::UnsupportedSchema)
                    }
                    return Ok(RemoteResource{
                        uri: uri_conversion.unwrap(),
                        credentials: creds,
                        target: write_target,
                        model: data_model,
                        cache: std::sync::OnceLock::new(),
                        no_cache: no_cache,
                        schema: schema
                    });
                }
                eprintln!("Resource with invalid url is ignored");
                return Err(ServerConfigError::InvalidValue)
            },
            Err(e) => {
                eprintln!("{}",e);
                Err(ServerConfigError::InvalidValue)
            }
        },
        Err(e) => {
            eprintln!("{}",e);
            Err(ServerConfigError::InvalidValue)
        }
    }
}