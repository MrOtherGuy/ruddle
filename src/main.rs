#![deny(warnings)]
use clap::{Parser,Subcommand,Args};
use std::sync::OnceLock;
use std::path::PathBuf;

mod server;
mod httpsconnector;
mod settings;
mod server_service;
mod models;
mod post_api;
mod service_response;
mod schemers;
mod content_type;

#[path = "./support/mod.rs"]
mod support;

use settings::Settings;

static SERVER_CONF : OnceLock<Settings> = OnceLock::new();

#[derive(Parser,Clone)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[arg(short, long)]
    port: Option<u16>,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(long)]
    start_in: Option<String>,

    #[arg(long)]
    no_console: bool,

    #[arg(short, long)]
    debug: bool,

    #[arg(short, long)]
    silent: bool,

    #[arg(short, long)]
    fast: bool,

    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Args, Debug,Clone)]
struct EncodeArgs {
   #[arg(long)]
   source: String,
   #[arg(long)]
   key: Option<String>
}

#[derive(Args, Debug,Clone)]
pub(crate) struct WebviewArgs {
   #[arg(long,default_value_t = 960)]
   width: u16,
   #[arg(long,default_value_t = 640)]
   height: u16,
   #[arg(long)]
   title: Option<String>
}

#[derive(Subcommand,Clone)]
enum Commands {
    /// Updates product list
    Update,
    /// Intentionally crashes the application
    Crash,
    /// Start the application normally
    Start,
    /// Create encoded form from string
    Encode(EncodeArgs),
    /// Display as webwiev
    Webview(WebviewArgs)
}

fn build_config(cli: Cli) -> Settings<'static>{
    if cli.fast {
        let c = config::Config::builder()
        .add_source(config::File::from_str(
r#"
port = 9000
server_root = "./"
resources = ["*"]
"#,
        config::FileFormat::Toml,
        ))
        .build()
        .unwrap();
        return Settings::from_config(c,&cli)
    }
    let binding = PathBuf::from("./Settings.toml");
    let config_file = match cli.config.as_deref(){
        Some(file) => file,
        None => binding.as_path()
    };
    match Settings::from_file(config_file,&cli){
        Ok(c) => c,
        Err(e) => {
            println!("{}",e);
            panic!("Configuration file content is invalid")
        }
    }
}

pub(crate) enum RuntimeMode{
    Headless,
    Normal,
    Webview(WebviewArgs)
}

//#[tokio::main]
pub fn main() -> () {
    
    let cli = Cli::parse();
    
    if cli.no_console{
        hide_console::hide_console();
    }
    
    let config = build_config(cli.clone());
    let conf = SERVER_CONF.get_or_init(|| config);
    let comm = match cli.command{
        Some(c) => c,
        None => Commands::Start
    };
    let mode = match comm {
        Commands::Webview(ref args) => RuntimeMode::Webview(args.clone()),
        _ => match cli.silent {
            true => RuntimeMode::Headless,
            false => RuntimeMode::Normal
        }
    };
    match comm {
        Commands::Crash => {
            panic!("Running 'crash' task");
        },
        Commands::Update => {
            println!("Running 'update' task");
            match server::update_task(&conf,mode){
                Ok(result) => {
                    print!("Data bytes: {:?}",result.data().unwrap().data_bytes().len())
                },
                Err(e) => eprintln!("{:?}",e)
            }
        },
        Commands::Start => {
            println!("Running 'start' task");
            server::start_server(&conf,mode).expect("Server failed");
            ()
        },
        Commands::Encode(args) => {
            println!("Running 'Encode task'");
            let text = crate::support::cryptea::encode_as_base64(&args.source,&args.key.unwrap_or("2.71828182845904".to_string())).unwrap();
            println!("{}",text);
            ()
        },
        Commands::Webview(_) => {
            println!("Running with webview");
            server::start_server(&conf,mode).expect("Server failed");
            
            ()
        }
    }
    ()
    
}

#[cfg(test)]
fn build_test_config() -> config::Config{
    let c = config::Config::builder()
    .add_source(config::File::from_str(
    r#"
port = 50242
server_root = "app"
resources = [
 "index.html",
 "js/test/",
 "css/",
 "favicon.ico"
]

[remote_resources]
bad = { url = ":example.com" }
disallowed = { url = "http://localhost:50242" }
thing = { url = "http://localhost", key_value = "eVz3/UsDq0w2nTXr89lDG20fd4bEWHiQAPIoSogQIqBhLtfX", key_header = "custom-header" }
missing = { url = "http://example.com", key_value = "eVz3/UsDq0w2nTXr89lDG20fd4bEWHiQAPIoSogQIqBhLtfX" }
"#,
    config::FileFormat::Toml,
    ))
    .build()
    .unwrap();
    c
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_uri() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("thing").unwrap().derive_key("2.71828182845904"){
            Ok(k) => assert_eq!(k,"Hello! This is my custom value here."),
            Err(_) => panic!("Decode key mismatch")
        }
    }
    #[test]
    fn test_codec(){
        use crate::support::cryptea;
        let input = "Testing string with whatever content";
        let key = "3.14159265358979";
        let encoded = cryptea::encode_as_base64(input,key).unwrap();
        println!("Length: {},{}",encoded.len(),encoded);
        let slice : [u8;48] = match encoded.as_bytes().try_into(){
            Ok(s) => s,
            Err(e) => {
                println!("{}",e);
                panic!("Got wrong slice")
            }
        };
        let res = match cryptea::decode(&slice,key){
            Ok(s) => s,
            Err(e) => {
                println!("{}",e);
                panic!("Decode failed")
            }
        };
        assert_eq!(input,&res);
    }
    #[test]
    fn partial_credentials() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("missing").unwrap().derive_key("2.71828182845904"){
            Ok(_) => panic!("Key decoding should have failed"),
            Err(e) => match e {
                settings::ServerConfigError::NotAvailable => (),
                _ => panic!("Wrong error thrown")
            }
        }
    }
      
    #[test]
    #[should_panic]
    fn reject_root_api() {
        let cli = Cli::parse();
        let config = config::Config::builder()
        .add_source(config::File::from_str(
r#"
port = 9000
server_root = "./api"
"#,
        config::FileFormat::Toml,
        ))
        .build()
        .unwrap();
        let _ = Settings::from_config(config,&cli);
        ()
    }
    #[test]
    fn bad_uri() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("bad"){
            Some(_k) => panic!("This shouldn't exist"),
            None => ()
        }
    }
    #[test]
    fn disallowed_uri() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("disallowed"){
            Some(_k) => panic!("This shouldn't exist"),
            None => ()
        }
    }
    #[test]
    fn bad_key() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("thing").unwrap().derive_key("2.71828182845905"){
            Ok(_) => panic!("Decoding with known bad key succeeded"),
            Err(_) => println!("DecodeError as expected")
        }
    }
    #[test]
    fn unknown_table() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        match settings.get_resource("unknown"){
            Some(_) => panic!("Decoding with known bad key succeeded"),
            None => ()
        }
    }
    #[test]
    fn resource_found() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        let resources = settings.resources.unwrap();
        assert!(resources.contains_path("/index.html"));
        assert!(resources.contains_path("/css/main.css"));
        assert!(resources.contains_path("/js/test/"));
    }

    #[test]
    fn resource_not_found() {
        let cli = Cli::parse();
        let config = build_test_config();
        let settings = Settings::from_config(config,&cli);
        let resources = settings.resources.unwrap();
        assert_eq!(resources.contains_path("/not_there.html"),false);
        assert_eq!(resources.contains_path("/js/test.js"),false);
    }

    #[test]
    fn fast_server(){
        let cli = Cli::parse();
        let config = config::Config::builder()
        .add_source(config::File::from_str(
r#"
port = 9000
server_root = "./"
resources = ["*"]
"#,
        config::FileFormat::Toml,
        ))
        .build()
        .unwrap();
        let settings = Settings::from_config(config,&cli);
        assert!(settings.resources.is_none())
    }

    #[test]
    fn test_with_model(){
        let cli = Cli::parse();
        let config = config::Config::builder()
        .add_source(config::File::from_str(
r#"
port = 9000
server_root = "./"

[remote_resources.update]
url = "https://example.com"
file_target = "./app/data/stored.json"
model = "text"
"#,
        config::FileFormat::Toml,
        ))
        .build()
        .unwrap();
        let settings = Settings::from_config(config,&cli);
        match settings.remote_resources.unwrap().get("update").unwrap().model{
            crate::models::RemoteResultType::RemoteTXT => (),
            _ => panic!("Incorrect RemoteBytes")
        }
    }
    #[test]
    fn test_with_schema(){
        let cli = Cli::parse();
        let config = config::Config::builder()
        .add_source(config::File::from_str(
r#"
port = 9000
schema_source = "test"
server_root = "./"
resources = ["*"]

[remote_resources.update]
url = "https://example.com"
model = "json"
schema = "test"
"#,
        config::FileFormat::Toml,
        ))
        .build()
        .unwrap();
        let settings = Settings::from_config(config,&cli);
        let tested_json = r#"{
"RequiredTest":[
{"test_code":"hello", "test_float": 4.5, "test_int": 1, "test_number": 32465476, "additional": "test"},
{"test_code":"hello", "test_float": 4.5, "test_int": 1, "test_number": 32465476}
]}
"#;
        let schema_name = match &settings.remote_resources{
            Some(r) => match r.get("update"){
                Some(res) => &res.schema,
                None => panic!("No resource")
            },
            None => panic!("No remote table")
        };
        let validator = &settings.get_schema(schema_name).unwrap();
        let result: serde_json::Result<serde_json::Value> = serde_json::from_str(tested_json);
        validator.validate(&result.unwrap()).unwrap()
        
        
    }

}