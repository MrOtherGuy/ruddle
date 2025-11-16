#![deny(warnings)]
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::{Method, Request, Response, StatusCode, HeaderMap};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::settings::resource::{RemoteResource,ResourceMethod};
use crate::settings::commandapi::ServerAPI;
use crate::SERVER_CONF;
use crate::httpsconnector::{request_optionally_validated_json,ConnectionError};
use crate::post_api::read_post_body;
use crate::models::{RemoteResultType,RemoteData};
use crate::service_response::ServiceResponse;
use crate::content_type::{NegotiationError,ContentType};


pub type HyperResponse = Response<BoxBody<Bytes, std::io::Error>>;
pub type HyperResult = hyper::Result<HyperResponse>;

static INDEX: &str = "/index.html";



pub enum ServerCommand<'a>{
    GetAPIRequest(&'a ServerAPI),
    PostAPIRequest(&'a ServerAPI)
}

fn command_task_resolved(json_data: RemoteData) -> HyperResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(json_data.into_bytes().into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

pub async fn run_command(command: &ServerCommand<'_>, request: Request<hyper::body::Incoming>) -> HyperResult {
    let conf = match SERVER_CONF.get(){
        Some(c) => c,
        None => return ServiceResponse::not_found()
    };
    
    let server_api = match command{
        ServerCommand::GetAPIRequest(comm) => comm,
        ServerCommand::PostAPIRequest(comm) => comm
    };
    if !server_api.has_required_headers(request.headers()){
        return ServiceResponse::bad_request()
    }
    if server_api.is_data(){
        return server_api.resolve_as_data()
    }
    // At this point, the command has to be a RequestCommand, and thus there MUST exist a corresponding resource
    let api_command = server_api.as_command().unwrap();
    let resource = conf.get_command_resource(api_command);

    match do_command_task(resource,conf,request).await{
        Ok(s) => Ok(command_task_resolved(s)),
        Err(_) => ServiceResponse::not_found()
    }
}

async fn do_command_task(resource : &RemoteResource, conf: &crate::Settings<'_>,request : Request<hyper::body::Incoming>) -> Result<RemoteData,ConnectionError>{
        match resource.get_cached(){
        Some(res) => {
            println!("Returning cached data");
            return Ok(res.clone())
        },
        None => ()
    };
    let data_kind = match &resource.model{
        RemoteResultType::RemoteJSON(kind) => kind.clone(),
        _ => return Err(ConnectionError::NotSupported)
    };
    let request_init = match resource.method{
        ResourceMethod::Get => resource.build_request(conf.user_agent.as_str(), request.uri().query(),None)?,
        ResourceMethod::Post => {
            let query = match request.uri().query(){
                Some(s) => Some(s.to_owned()),
                None => None
            };
            let body = read_post_body(request).await;
            if let Err(e) = body{
                eprintln!("{e}");
                return Err(ConnectionError::InvalidRequest)
            }
            // Should maybe check against schema or something
            resource.build_request(conf.user_agent.as_str(), query.as_deref(),Some(body.unwrap().into()))?
        }
    };

    match request_optionally_validated_json(request_init, data_kind, conf.get_schema(&resource.schema)).await{
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
            if resource.no_cache {
                return Ok(r)
            }
            println!("Inserting to cache...");
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