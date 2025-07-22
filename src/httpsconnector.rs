#![deny(warnings)]
use bytes::Bytes;
use http_body_util::{Full,Empty,BodyExt,Collected};
use hyper_tls::HttpsConnector;
use hyper::{body::Buf,Request};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

use crate::models::{RemoteResult,RemoteData,JSONSerializeType,JSONKind};
use crate::schemers::validator::Validator;
use crate::settings::resource::{ResourceMethod,RequestCredentials};
use crate::settings::header::HeaderSet;

pub type ConnectionResult<T> = Result<T, ConnectionError>;

#[allow(unused)]
#[derive(Debug)]
pub enum ConnectionError{
    NotFound,
    InvalidURI,
    InvalidJSON,
    NoFrame,
    InvalidUTF8,
    InvalidRequest,
    InternalError,
    NotSupported
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            ConnectionError::NotFound => write!(f, "Resource not found"),
            ConnectionError::InvalidURI => write!(f, "Invalid URI"),
            ConnectionError::InvalidJSON => write!(f, "Invalid JSON"),
            ConnectionError::NoFrame => write!(f, "Invalid HTTP frame"),
            ConnectionError::InvalidUTF8 => write!(f, "Invalid UTF8"),
            ConnectionError::InvalidRequest => write!(f, "Request could not be constructed"),
            ConnectionError::InternalError => write!(f, "Server found itself from an unexpected state"),
            ConnectionError::NotSupported => write!(f, "Requested data model is currently not supported")
        }
    }
}

pub async fn request_get_resource(request_init: RequestOptions<'_>) -> ConnectionResult<Collected<bytes::Bytes>>{
    let mut builder = Request::builder()
        .method(hyper::Method::GET)
        .uri(request_init.uri.clone())
        .header("User-Agent",request_init.user_agent);
    builder = match &request_init.credentials{
        Some(cred) => builder.header(&cred.key, &cred.value),
        None => builder
    };
    for header in request_init.request_headers.headers().iter(){
        builder = builder.header(header.name().as_str(), header.value().to_value_str());
    }
    let request = match builder.body(Empty::new()){
        Ok(req) => req,
        Err(_) => return Err(ConnectionError::InvalidRequest)
    };
    let https = HttpsConnector::new();
    let client = Client::builder(TokioExecutor::new()).build::<_, Empty<Bytes>>(https);
    let res = match client.request(request).await{
        Ok(r) => r,
        Err(e) => {
            println!("{}",e);
            return Err(ConnectionError::NotFound)
        }
    };

    if !res.status().is_success(){
        return Err(ConnectionError::NotFound)
    }

    let body = match res.collect().await{
      Ok(s) => s,
      Err(_) => return Err(ConnectionError::NotFound)  
    };
    Ok(body)
}

pub async fn request_post_resource(request_init: RequestOptions<'_>) -> ConnectionResult<Collected<bytes::Bytes>>{
    let mut builder = Request::builder()
        .method(hyper::Method::POST)
        .uri(request_init.uri.clone())
        .header("User-Agent",request_init.user_agent);
    builder = match &request_init.credentials{
        Some(cred) => builder.header(&cred.key, &cred.value),
        None => builder
    };
    let request = match builder.body(Full::new(request_init.bytes())){
        Ok(req) => req,
        Err(_) => return Err(ConnectionError::InvalidRequest)
    };
    let https = HttpsConnector::new();
    let client =  Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
    let res = match client.request(request).await{
        Ok(r) => r,
        Err(e) => {
            println!("{}",e);
            return Err(ConnectionError::NotFound)
        }
    };

    if !res.status().is_success(){
        return Err(ConnectionError::NotFound)
    }

    let body = match res.collect().await{
      Ok(s) => s,
      Err(_) => return Err(ConnectionError::NotFound)  
    };
    Ok(body)
}

pub struct RequestOptions<'a>{
    pub user_agent: &'a String,
    pub uri: hyper::Uri,
    pub credentials: Option<RequestCredentials>,
    pub method: &'a ResourceMethod,
    pub request_headers: &'a HeaderSet,
    pub body: Option<Bytes>
}

impl<'a> RequestOptions<'a>{
    fn bytes(self) -> Bytes {
        match self.body{
            Some(bytes) => bytes,
            None => Bytes::new()
        }
    }
}

pub fn validate_response(data_buffer: impl Buf, validator: &Validator) -> Result<RemoteData,ConnectionError>{

    match RemoteResult::json_with_schema(data_buffer,&JSONSerializeType::Pretty,validator){
        Ok(blob) => Ok(blob),
        Err(e) => {
            println!("{}",e);
            return Err(ConnectionError::InvalidJSON)
        }
    }
}

pub async fn request_optionally_validated_json(request_init: RequestOptions<'_>, data_kind: JSONKind, validator: Option<&Validator>) -> ConnectionResult<RemoteData>{
    let response = match request_json(request_init).await{
        Ok(res) => res.aggregate(),
        Err(_) => return Err(ConnectionError::InvalidJSON)
    };
    match (validator, data_kind){
        (Some(schema), JSONKind::UntypedValue) => validate_response(response,schema),
        (_,kind) => match RemoteResult::json(response,&kind,&JSONSerializeType::Pretty){
            Ok(blob) => Ok(blob),
            Err(e) => {
                println!("{}",e);
                Err(ConnectionError::InvalidJSON)
            }
        }
    }
}



pub async fn request_json(request_init: RequestOptions<'_>) -> ConnectionResult<Collected<Bytes>>{
    
    match &request_init.method{
        ResourceMethod::Get => match request_get_resource(request_init).await{
            Ok(s) => Ok(s),
            Err(e) => return Err(e)
        },
        ResourceMethod::Post => match request_post_resource(request_init).await{
            Ok(s) => Ok(s),
            Err(e) => return Err(e)
        }
    }
    
}