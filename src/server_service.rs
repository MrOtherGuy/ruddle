#![deny(warnings)]
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::{Method, Request, Response, StatusCode, HeaderMap};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::SERVER_CONF;
use crate::httpsconnector::{RequestOptions,request_json,ConnectionError};
use crate::models::RemoteData;
use crate::service_response::ServiceResponse;
use crate::content_type::{NegotiationError,ContentType};

pub type HyperResponse = Response<BoxBody<Bytes, std::io::Error>>;
pub type HyperResult = hyper::Result<HyperResponse>;

static INDEX: &str = "/index.html";



pub enum ServerCommand{
    Update,
    Thing
}

impl ServerCommand{
    pub fn from_str(input: &str) -> Option<Self>{
        match input{
            "update" => Some(Self::Update),
            "thing" => Some(Self::Thing),
            _ => None
        }
    }
    pub fn name(&self) -> &str{
        match self{
            ServerCommand::Update => "update",
            ServerCommand::Thing => "thing"
        }
    }
}

fn data_updated_handler(json_data: RemoteData) -> HyperResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(json_data.into_bytes().into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

pub async fn run_command(command: &ServerCommand, headers: &HeaderMap) -> HyperResult {
    let conf = match SERVER_CONF.get(){
        Some(c) => c,
        None => return ServiceResponse::not_found()
    };
    if !conf.has_required_headers(headers){
        return ServiceResponse::bad_request()
    }
    match update_task(command.name(),conf).await{
        Ok(s) => Ok(data_updated_handler(s)),
        Err(_) => ServiceResponse::not_found()
    }
}

pub async fn file_serve( req: Request<hyper::body::Incoming>) -> HyperResult {
    
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") | (&Method::GET, "/index.html") => simple_file_send(INDEX,req.headers()).await,
        (&Method::GET,path) => simple_file_send(path,req.headers()).await,
        _ => ServiceResponse::not_found(),
    }
}

async fn simple_file_send(filename: &str, headers: &HeaderMap) -> HyperResult {
    use futures_util::TryStreamExt;
    let config = match SERVER_CONF.get(){
        Some(c) => c,
        None => return ServiceResponse::not_found()
    };
    if !config.can_read_resource(filename){
        return ServiceResponse::not_found()
    }
    
    // Open file for reading
    
    let file = File::open(format!("{}{}",config.server_root,filename)).await;
    if file.is_err() {
        eprintln!("ERROR: Unable to open file: {}",filename);
        return ServiceResponse::not_found();
    }

    let file: File = file.unwrap();
    let content_type : ContentType = match filename.parse(){
        Ok(t) => t,
        Err(_) => ContentType::Unknown 
    };

    let reader_stream = ReaderStream::new(file);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    match content_type.into_response(&config,headers){
        Ok(builder) => Ok( builder.status(StatusCode::OK).body(boxed_body).unwrap() ),
        Err(e) => match e{
            NegotiationError::NotAcceptable => ServiceResponse::not_acceptable()
        }
    }
    
}

async fn update_task(resource_name : &str, conf: &crate::Settings<'_>) -> Result<RemoteData,ConnectionError>{
    let resource = match conf.get_resource(resource_name){
        Some(res) => res,
        None => return Err(ConnectionError::InvalidRequest)
    };
    match resource.get_cached(){
        Some(res) => {
            println!("Returning cached data");
            return Ok(res.clone())
        },
        None => ()
    };
    let creds = match resource.request_credentials("2.718281828459045"){
        Some(res) => match res{
            Ok(dec) => Some(dec),
            Err(_) => return Err(ConnectionError::InvalidRequest)
        },
        None => None
    };
    let request_init = RequestOptions{
        uri: resource.uri.uri().clone(),
        credentials: creds,
        user_agent: &conf.user_agent,
        model: resource.model.clone(),
        schema: conf.get_schema(&resource.schema)
    };
    match request_json(request_init).await{
        Ok(r) => {
            match &resource.target{
                Some(res) => {
                    match res.write_file(&r).await{
                        Ok(_) => println!("file saved!"),
                        Err(e) => eprintln!("{:?}",e)
                    };
                    ()
                },
                None => ()
            }
            println!("Inserting to cache...");
            if resource.no_cache {
                return Ok(r)
            }
            match resource.cache_result(r){
                Ok(r) => Ok(r),
                Err(_) => Err(ConnectionError::InternalError)
            }
        },
        Err(e) => {
          println!("{}",e);
          Err(e)
        }
    }
}