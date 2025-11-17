#![deny(warnings)]

use config::Config;
use std::collections::{HashSet,HashMap};
use std::path::Path;

use crate::Commands;
use crate::schemers::{schemaloader::{build_test,SchemaTree},validator::Validator};
use crate::content_type::{ContentType,HeaderMap,GetHeaderValueString};

pub(crate) mod header;
mod pathprovider;
pub mod resource;
mod qualifieduri;
mod credentials;
pub(crate) mod commandapi;

pub(crate) use header::{HeaderValue,HeaderSet,ParseMode};
use commandapi::{ServerAPI,CommandAPI,RequestCommand};
use pathprovider::PathProvider;
use resource::{ResourceStore,RemoteResource,TryParseTypedValue};

pub type ServerConfigResult<T> = Result<T, ServerConfigError>;
impl std::fmt::Display for ServerConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            ServerConfigError::InvalidError => write!(f, "Configuration file is invalid"),
            ServerConfigError::NotFoundError => write!(f, "Configuration file could not be found"),
            ServerConfigError::InvalidFileListError => write!(f, "Couldn't parse file list"),
            ServerConfigError::InvalidPathsError => write!(f, "Couldn't construct file paths"),
            ServerConfigError::UnknownError =>  write!(f,"Unknown error occured"),
            ServerConfigError::DecodeError => write!(f,"Key decoding failed"),
            ServerConfigError::InvalidValue => write!(f,"Value could not be parsed"),
            ServerConfigError::MissingKey => write!(f,"Key doesn't exist"),
            ServerConfigError::NotAvailable => write!(f,"Key is not available"),
            ServerConfigError::InvalidURI => write!(f,"Given uri doesn't is not valid to use"),
            ServerConfigError::UnsupportedSchema => write!(f,"Schema is only supported for 'untypedvalue'"),
            ServerConfigError::InvalidSchema => write!(f,"Schema source is not valid"),
            ServerConfigError::NoSchemaSource => write!(f,"Schema source file could not be loaded")
        }
    }
}
#[allow(unused)]
#[derive(Debug)]
pub enum ServerConfigError{
    
    InvalidError,
    NotFoundError,
    InvalidFileListError,
    InvalidPathsError,
    UnknownError,
    DecodeError,
    InvalidValue,
    MissingKey,
    NotAvailable,
    InvalidURI,
    UnsupportedSchema,
    InvalidSchema,
    NoSchemaSource
}

pub struct WebviewOptions{
    pub width: u16,
    pub height: u16,
    pub title: String,
    pub show_console: bool
}

impl Default for WebviewOptions{
    fn default() -> Self{
        WebviewOptions{ width: 960, height: 640, title: "Rusty-labels".to_string() , show_console: false }
    } 
}
pub enum RuntimeMode{
    Headless,
    Normal,
    Webview(WebviewOptions)
}

impl WebviewOptions{
    fn from_config_and_cli(config: &Config, cli: crate::WebviewArgs) -> Self{
        match config.get_table("webview"){
            Ok(table) => {
                let width = match (cli.width,table.try_parse_u16("width")){
                    (Some(a),_) => Some(a),
                    (None,Ok(w)) => if w > 100 && w < 3000{
                        Some(w)
                    }else{
                        None
                    },
                    (None,Err(_)) => None
                };
                let height = match (cli.height,table.try_parse_u16("height")){
                    (Some(a),_) => Some(a),
                    (None,Ok(w)) => if w > 100 && w < 3000{
                        Some(w)
                    }else{
                        None
                    },
                    (None,Err(_)) => None
                };
                let title = match (cli.title,table.try_parse_string("title")){
                    (Some(t),_) => t,
                    (None,Ok(title)) => title,
                    (None,Err(_)) => "Rusty-labels".to_string()
                };
                let show_console = match (cli.show_console,table.try_parse_bool("show_console")){
                    (true,_) => true,
                    (false,Ok(a)) => a,
                    (false,Err(_)) => false
                };
                match (width,height){
                    (Some(width),Some(height)) => WebviewOptions{ width, height, title, show_console },
                    (_,_) => WebviewOptions::default()
                }
            },
            Err(_) => WebviewOptions{
                width: cli.width.unwrap_or(960),
                height: cli.height.unwrap_or(640),
                title: cli.title.unwrap_or("Rusty-labels".to_string()),
                show_console: cli.show_console
            }
        }
    }
}

pub(crate) fn parse_config_string_table(config : &HashMap<String, config::Value>, table_name: &str) -> Option<HashMap<String,String>>{
    match config.get(table_name){
        Some(t) => match t.clone().into_table(){
            Ok(table) => {
                let mut map = HashMap::new();
                for (key,val) in table.iter(){
                    if let Ok(value) = val.clone().into_string(){
                        map.insert(key.clone(),value);
                    };
                };
                match map.is_empty(){
                    true => None,
                    false => Some(map)
                }
            },
            Err(_) => None
        },
        None => None
    }
}

pub(crate) fn merge_string_maps(mut owned: HashMap<String,String>, ref_map: &HashMap<String,String>) -> HashMap<String,String>{
    for (key,val) in ref_map.iter(){
        if !owned.contains_key(key){
            owned.insert(key.clone(),val.clone());
        }
    }
    owned
}

pub struct Settings<'a>{
    pub port: u16,
    pub run_mode: RuntimeMode,
    pub subcommand: Option<Commands>,
    pub server_root: String,
    pub start_in: Option<String>,
    pub resources: Option<PathProvider<'a>>,
    pub writable_resources: Option<PathProvider<'a>>,
    pub user_agent: String,
    remote_resources: Option<ResourceStore>,
    pub header_map: HashMap<ContentType,HashMap<String,HeaderValue>>,
    schema_tree: Option<SchemaTree>,
    pub allow_origins: HashSet<String>,
    api_required_headers: Option<HashMap<String,String>>,
    commands: Option<CommandAPI>
}



impl Settings<'_>{
    pub fn has_required_headers(&self, request_headers: &hyper::HeaderMap) -> bool{
        if let Some(required) = &self.api_required_headers{
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
    pub fn has_console(&self) -> bool{
        match &self.run_mode{
            RuntimeMode::Webview(opts) => opts.show_console,
            _ => true
        }
    }
    pub fn can_read_resource(&self, path: &str) -> bool{
        match &self.resources{
            None => true,
            Some(pp) => pp.contains_path(path)
        }
    }
    pub fn can_write_resource(&self, path: &str) -> bool{
        match &self.writable_resources{
            None => false,
            Some(pp) => pp.contains_path(path)
        }
    }
    pub fn get_schema(&self,schema_name: &Option<String>) -> Option<&Validator>{
        if schema_name.is_none(){
            return None
        }
        let name = match schema_name{
            Some(n) => n.trim_start_matches("."),
            None => return None
        };
        match &self.schema_tree{
            Some(tree) => tree.get_schema(&name),
            None => None
        }
    }
    pub fn get_command_resource(&self, request_command: &RequestCommand) -> &RemoteResource{
        let rr = self.remote_resources.as_ref().unwrap();
        rr.get_command_resource(request_command)
    }
    pub fn maybe_panics_get_command_resource(&self, request_command: &RequestCommand) -> Option<&RemoteResource>{
        let rr = match &self.remote_resources{
            Some(rr) => rr,
            None => return None
        };
        Some(rr.get_command_resource(request_command))
    }
    pub fn get_api(&self, resource_name: &str) -> Option<&ServerAPI>{
        let commands = match &self.commands{
            Some(comms) => comms,
            None => return None
        };
        commands.get_api(resource_name)
    }
    pub fn post_api(&self, resource_name: &str) -> Option<&ServerAPI>{
        let commands = match &self.commands{
            Some(comms) => comms,
            None => return None
        };
        commands.post_api(resource_name)
    }
    pub fn from_config(config: Config, cli: crate::Cli) -> Settings<'static>{
        let port_number = match cli.port{
            Some(p) => p,
            None => config.get::<u16>("port").unwrap_or(8080)
        };
        let run_mode = match &cli.command{
            Some(c) => {
                let options = match c{
                    Commands::Webview(a) => WebviewOptions::from_config_and_cli(&config, a.clone()),
                    _ => WebviewOptions::from_config_and_cli(&config,crate::WebviewArgs::empty())
                };
                RuntimeMode::Webview(options)
            },
            None => match config.get::<String>("run_mode"){
                Ok(s) => match s.as_str(){
                    "headless" => RuntimeMode::Headless,
                    "webview" => {
                        let options = WebviewOptions::from_config_and_cli(&config,crate::WebviewArgs::empty());
                        RuntimeMode::Webview(options)
                    },
                    _ => RuntimeMode::Normal
                },
                Err(_) => RuntimeMode::Normal
            }

        };
        let start_in = match &cli.start_in {
            Some(s) => Some(s.clone()),
            None => match config.get::<String>("start_in"){
                Ok(s) => Some(s),
                Err(_) => None
            }
        };
        let schema_filename = match config.get::<String>("schema_source"){
            Ok(s) => Some(s),
            Err(_) => None
        };
        
        let schema_source = match schema_filename{
            Some(s) => {
                let result = match s.as_str(){
                    "test" => build_test(),
                    a => SchemaTree::load_from_file(a)
                };
                match result {
                    Ok(tree) => Some(tree),
                    Err(_) => panic!("Specified schema source could not be loaded!")
                }
            },
            None => None
        };
        let allow_origins = match config.get_array("allow_origins"){
            Ok(list) => {
                let mut set = HashSet::new();
                list.into_iter().for_each(|val| {
                    if let Ok(s) = val.into_string(){
                        set.insert(s.clone());
                    }
                });
                set
            },
            Err(e) => {
                println!("{}",e);
                HashSet::new()
            }
        };
        let resources = match config.get_array("resources"){
            Ok(list) => PathProvider::from_iter(list.into_iter()),
            Err(e) => {
                println!("{}",e);
                None
            }
        };
        let write_resources = match config.get_array("writable_resources"){
            Ok(list) => PathProvider::from_iter(list.into_iter()),
            Err(e) => {
                println!("{}",e);
                None
            }
        };
        
        
        let api_requirements : Option<HashMap<String,String>> = match config.get_table("api_required_headers"){
            Ok(table) => match table.is_empty(){
                true => None,
                false => {
                    let mut hmap : HashMap<String,String> = HashMap::new();
                    for (key,val) in table.iter(){
                        if let Ok(string_value) = val.clone().into_string(){
                            hmap.insert(key.to_string(),string_value);
                        }
                    }
                    Some(hmap)
                }
            },
            Err(e) => {
                println!("{}",e);
                None
            }
        };
        let remote_store = match config.get_table("remote_resources"){
            Ok(s) => match s.is_empty(){
                true => None,
                false => match ResourceStore::try_parse(&s,&schema_source, port_number){
                    Ok(store) => Some(store),
                    Err(_) => None
                },
            },
            Err(_) => None

        };
        
        let headers : HeaderMap = match config.get_table("response_headers"){
            Ok(s) => {

                let mut map = HashMap::new();
                
                for (key,val) in s.iter(){
                    let mime_type = ContentType::from_mime_type(&key);
                    match HeaderSet::parse(&val,ParseMode::IgnoreInvalid){
                        Ok(heads) => {
                            let inner_map : HashMap<String,HeaderValue> = HashMap::from_iter(heads.into_iter().map(|h| h.into_pair()));
                            map.insert(mime_type,inner_map);
                            ()
                        },
                        Err(e) => {
                          eprintln!("{e}");
                          ()
                        }
                    }
                };
                if let Some(globals) = map.remove(&ContentType::Global){
                    map.iter_mut().for_each(|(_,spec_item)| {
                        globals.iter().for_each(|(key,val)| if !spec_item.contains_key(key){
                            spec_item.insert(key.clone(),val.clone());
                        })
                    });
                    map.insert(ContentType::Global,globals);
                };
                map
            },
            Err(_) => HashMap::new()
        };
        let ref_map = match &remote_store{
            Some(store) => store.inner(),
            None => &HashMap::new()
        };
        let commands = CommandAPI::try_parse(&config,&ref_map,&api_requirements);
        let root = config.get::<String>("server_root").unwrap_or("server_root".to_string());
        if root.starts_with("api/") || root.starts_with("./api/") || root == "api" || root == "./api"{
            panic!("Server root directory must not be named 'api'");
        }
        Settings{
            port: port_number,
            server_root: root,
            start_in: start_in,
            allow_origins: allow_origins,
            writable_resources: write_resources,
            resources: resources,
            user_agent: config.get::<String>("user_agent").unwrap_or("curl/7.54.1".to_string()),
            remote_resources: remote_store,
            header_map: headers,
            schema_tree: schema_source,
            commands,
            api_required_headers: api_requirements,
            run_mode,
            subcommand: cli.command
        }
    }
    pub fn from_file(filename: &Path,cli: crate::Cli) -> ServerConfigResult<Settings<'static>>{
        let config_file = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name(match filename.to_str(){
            Some(s) => s,
            None => "Settings.toml"
        }))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("APP"))
        .build();
        
        let config = match config_file{
            Ok(s) => s,
            Err(e) => {println!("{e}");return match e{
                config::ConfigError::FileParse{uri: _, cause: _} => Err(ServerConfigError::InvalidError),
                _ => {
                    println!("{}",e);
                    Err(ServerConfigError::NotFoundError)
                }}
            }
        };
        Ok(Settings::from_config(config,cli))
    }
    

}


