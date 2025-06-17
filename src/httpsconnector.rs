#![deny(warnings)]
use bytes::Bytes;
use http_body_util::{Empty,BodyExt};
use hyper_tls::HttpsConnector;
use hyper::{body::Buf,Request};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

use crate::models::{RemoteResult,RemoteResultType,RemoteData,JSONSerializeType,JSONKind};
use crate::schemers::validator::Validator;
use crate::settings::resource::RequestCredentials;

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

pub async fn request_resource(request_init: &RequestOptions<'_>) -> ConnectionResult<impl Buf>{
    let https = HttpsConnector::new();
    let client = Client::builder(TokioExecutor::new()).build::<_, Empty<Bytes>>(https);
    let mut builder = Request::builder()
        .method(hyper::Method::GET)
        .uri(request_init.uri.clone())
        .header("User-Agent",request_init.user_agent);
    builder = match &request_init.credentials{
        Some(cred) => builder.header(&cred.key, &cred.value),
        None => builder
    };
    let request = match builder.body(Empty::new()){
        Ok(req) => req,
        Err(_) => return Err(ConnectionError::InvalidRequest)
    };
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
    Ok(body.aggregate())
}


pub struct RequestOptions<'a>{
    pub user_agent: &'a String,
    pub uri: hyper::Uri,
    pub credentials: Option<RequestCredentials>,
    pub model: RemoteResultType,
    pub schema: Option<&'a Validator>
}

pub async fn request_json(request_init: RequestOptions<'_>) -> ConnectionResult<RemoteData>{
    let data_kind = match request_init.model{
        RemoteResultType::RemoteJSON(ref kind) => kind.clone(),
        _ => return Err(ConnectionError::NotSupported)
    };
    let data_buffer = match request_resource(&request_init).await{
        Ok(s) => s,
        Err(e) => {
            return Err(e)
        }
    };
    match (&data_kind,request_init.schema.is_none()) {
        (JSONKind::UntypedValue,false) => {
            match RemoteResult::json_with_schema(data_buffer,&JSONSerializeType::Pretty,request_init.schema.unwrap()){
                Ok(blob) => Ok(blob),
                Err(e) => {
                    println!("{}",e);
                    return Err(ConnectionError::InvalidJSON)
                }
            }
        },
        (_,_) => match RemoteResult::json(data_buffer,&data_kind,&JSONSerializeType::Pretty){
            Ok(blob) => Ok(blob),
            Err(e) => {
                println!("{}",e);
                return Err(ConnectionError::InvalidJSON)
            }
        }
    }
    
}