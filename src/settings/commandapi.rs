#![deny(warnings)]

use std::collections::HashMap;
use crate::server_service::HyperResult;
use crate::settings::{ServerConfigError,parse_config_string_table,merge_string_maps};
use super::resource::{RemoteResource,TryParseTypedValue};
use crate::content_type::{GetHeaderValueString};
use hyper::{StatusCode,Response};

#[derive(Debug)]
enum ServerAPIType{
    Get(ServerAPI),
    Post(ServerAPI)
}
#[derive(Debug)]
enum APIResponseType{
    Command(RequestCommand),
    Data(DataCommand)
}
type ConfigMap = config::Map<String, config::Value>;
impl ServerAPIType{
    fn try_from_table(table: &ConfigMap,command_api: ServerAPI) -> Result<Self,ServerConfigError>{
        match table.try_parse_string("method"){
            Ok(met) => match met.as_str(){
                "get" | "GET" => Ok(ServerAPIType::Get(command_api)),
                "post" | "POST" => Ok(ServerAPIType::Post(command_api)),
                _ => Err(ServerConfigError::InvalidValue)
            },
            Err(_) => Err(ServerConfigError::MissingKey)
        }
    }
}

pub type APIMap = HashMap<String,ServerAPI>;

#[derive(Debug)]
pub struct CommandAPI{
    post: APIMap,
    get: APIMap
}

#[derive(Debug)]
pub struct RequestCommand{
    name: String
}

impl RequestCommand{
    // Note! Program logic expects that a matching resource can definitely be found,
    // so if you construct this and try to use it, then panics can happen.
    pub fn new(name: &str) -> Self{
        RequestCommand{ name: name.to_owned() }
    }
    pub fn name(&self) -> &str{
        self.name.as_str()
    }
}

impl CommandAPI{
    pub fn get_api(&self,api_name: &str) -> Option<&ServerAPI>{
        self.get.get(api_name)
    }
    pub fn post_api(&self, api_name: &str) -> Option<&ServerAPI>{
        self.post.get(api_name)
    }
    pub fn try_parse(conf: &config::Config, available_remotes: &HashMap<String, RemoteResource>, global_required_headers : &Option<HashMap<String,String>>) -> Option<Self>{
        let command_apis : Option<CommandAPI> = match conf.get_table("apis"){
            Ok(table) => match table.is_empty(){
                true => None,
                false => {
                    let mut get_map : APIMap = HashMap::new();
                    let mut post_map : APIMap = HashMap::new();
                    for (key,val) in table.iter(){
                        if let Ok(server_api) = ServerAPI::try_parse(val,available_remotes,global_required_headers){
                            match server_api{
                                ServerAPIType::Get(api) => get_map.insert(key.to_string(),api),
                                ServerAPIType::Post(api) => post_map.insert(key.to_string(),api)
                            };
                        };
                    }
                    Some(CommandAPI{
                        get: get_map,
                        post: post_map
                    })
                }
            },
            Err(e) => {
                println!("{}",e);
                None
            }
        };
        command_apis
    }
}

#[derive(Debug)]
pub struct ServerAPI{
    response_type: APIResponseType,
    required_headers: Option<HashMap<String,String>>
}

#[derive(Debug)]
struct DataCommand{
    response_code: StatusCode,
    content_type: String,
    value: Vec<u8>
}

impl DataCommand{
    pub fn resolve_into_response(&self) -> HyperResult{
        use http_body_util::{BodyExt, Full};
        Ok(Response::builder()
            .status(self.response_code)
            .header("Content-Type",self.content_type.as_str())
            .body(Full::new(self.value.clone().into()).map_err(|e| match e {}).boxed())
            .unwrap())
    }
    fn try_from_table(config_val : &config::Value) -> Result<Self,ServerConfigError>{
        let table = match config_val.clone().into_table(){
            Ok(t) => t,
            Err(_) => return Err(ServerConfigError::MissingKey)
        };
        let value = match table.try_parse_string("value"){
            Ok(s) => s.into_bytes(),
            Err(_) => return Err(ServerConfigError::MissingKey)
        };
        let content_type = match table.try_parse_string("type"){
            Ok(s) => s,
            Err(_) => return Err(ServerConfigError::MissingKey)
        };
        let response_code : hyper::StatusCode = match table.get("code"){
            Some(s) => match s.clone().into_uint(){
                Ok(u) => {
                    let uint : u16 = match u.try_into(){
                        Ok(u) => u,
                        Err(_) => return Err(ServerConfigError::InvalidValue)
                    };
                    match hyper::StatusCode::from_u16(uint){
                        Ok(code) => code,
                        Err(_) => return Err(ServerConfigError::InvalidValue)
                    }
                },
                Err(_) => return Err(ServerConfigError::InvalidValue)
            },
            None => return Err(ServerConfigError::MissingKey)
        };
        Ok(DataCommand{
            value,
            content_type,
            response_code
        })
    }
}

impl ServerAPI{
    pub fn resolve_as_data(&self) -> HyperResult{
        use crate::service_response::ServiceResponse;
        match &self.response_type {
            APIResponseType::Data(data_comm) => data_comm.resolve_into_response(),
            _ => {
                eprintln!("Server logic error");
                ServiceResponse::not_found_empty()
            }
        }
    }
    pub fn is_data(&self) -> bool{
        match &self.response_type {
            APIResponseType::Data(_) => true,
            _ => false
        }
    }
    pub fn has_required_headers(&self, request_headers: &hyper::HeaderMap) -> bool{
        if let Some(required) = &self.required_headers{
            for (key,val) in required.iter(){
                match request_headers.get_as_string(key){
                    Some(hv) => if hv != val{
                        return false
                    },
                    None => return false
                }
            }
            return true
        }
        return true
    }
    fn with_headers(response_type: APIResponseType, own_headers: Option<HashMap<String,String>>,global_required_headers : &Option<HashMap<String,String>>) -> ServerAPI{
        let map = match global_required_headers{
            Some(global) => match own_headers{
                Some(o) => Some(merge_string_maps(o,global_required_headers.as_ref().unwrap())),
                None => {
                    let mut map = HashMap::new();
                    for (key,val) in global.iter(){
                        map.insert(key.clone(),val.clone());
                    };
                    Some(map)
                }
            },
            None => own_headers
        };
        ServerAPI{
            response_type,
            required_headers: map
        }
    }
    pub fn as_command(&self) -> Option<&RequestCommand>{
        match &self.response_type{
            APIResponseType::Command(command) => Some(command),
            _ => None
        }
    }
    fn try_parse(conf: &config::Value, available_remotes: &HashMap<String, RemoteResource>, global_required_headers : &Option<HashMap<String,String>>) -> Result<ServerAPIType,ServerConfigError>{
        match conf.clone().into_table(){
            Ok(table) => match table.try_parse_string("command"){
                Ok(string) => match available_remotes.contains_key(&string){
                    true => {
                        let own_headers = parse_config_string_table(&table,"require_headers");
                        let command = ServerAPI::with_headers(APIResponseType::Command(RequestCommand::new(string.as_str())), own_headers, global_required_headers);
                        ServerAPIType::try_from_table(&table,command)
                    },
                    false => return Err(ServerConfigError::NotAvailable)
                },
                Err(_) => {
                    match table.get("response"){
                        Some(res) => match DataCommand::try_from_table(res){
                            Ok(command) => {
                                let own_headers = parse_config_string_table(&table,"require_headers");
                                let sapi = ServerAPI::with_headers(APIResponseType::Data(command), own_headers, global_required_headers);
                                ServerAPIType::try_from_table(&table,sapi)
                            },
                            Err(e) => Err(e)
                        },
                        None => Err(ServerConfigError::MissingKey)
                    }
                }
            },
            Err(e) => {
                eprintln!("{e}");
                Err(ServerConfigError::InvalidValue)
            }
        }
    }
}