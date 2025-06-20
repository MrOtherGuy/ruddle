#![deny(warnings)]
use hyper::{Request,Method,StatusCode,Response};
use hyper::body::Body;
use crate::service_response::ServiceResponse;
use crate::server_service::HyperResult;
use crate::models::{RemoteResult,JSONKind};
use http_body_util::{BodyExt, Full};

pub async fn handle_post_api( req: Request<hyper::body::Incoming>) -> HyperResult {
    let config = match crate::SERVER_CONF.get(){
        Some(c) => c,
        None => return ServiceResponse::bad_request()
    };
    if !config.has_required_headers(req.headers()){
        return ServiceResponse::bad_request()
    }
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/api/test") => test_post_api_response().await,
        (&Method::POST, "/api/post") => post_api_response(req).await,
        (&Method::POST, "/api/save") => try_save_data(req).await,
        _ => ServiceResponse::bad_request(),
    }
}

async fn try_save_data(req: Request<hyper::body::Incoming>) -> HyperResult{
    
    let uri_clone = req.uri().clone();
    let filename = match uri_clone.query(){
        Some(name) => name,
        None => return ServiceResponse::bad_request()
    };
    let config = match crate::SERVER_CONF.get(){
        Some(c) => c,
        None => return ServiceResponse::bad_request()
    };
    if !config.can_write_resource(&filename){
        return ServiceResponse::not_found_empty()
    }
    let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
    
    if upper > 1024 * 512 { // 512kB
        return ServiceResponse::content_too_large()
    }
    let body = match read_post_body(req).await{
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{}",e);
            return ServiceResponse::bad_request()
        }
    };
    match write_file(filename,&config.server_root,body).await{
        Ok(_) => ServiceResponse::created(),
        Err(e) => {
            eprintln!("{}",e);
            ServiceResponse::bad_request()
        }
    }
}

pub async fn read_post_body(req: Request<hyper::body::Incoming>) -> Result<Vec<u8>,std::io::Error>{
    use bytes::Buf;
    use std::io::Read;
    use std::io::{Error,ErrorKind};
    let buffer = match req.collect().await{
        Ok(body) => body.aggregate(),
        Err(e) => {
            eprintln!("{}",e);
            return Err(Error::from(ErrorKind::UnexpectedEof))
        }
    };
    let mut reader = buffer.reader();
    let mut m_vec : Vec<u8> = vec![];
    match reader.read_to_end(&mut m_vec) {
        Ok(_) => Ok(m_vec),
        Err(e) => Err(e)
    }
}

async fn write_file(filename : &str,root: &str,stream: Vec<u8>) -> Result<(),std::io::Error>{
    use tokio::io::AsyncWriteExt;
    let path = format!("{}{}",root,filename);
    println!("Writing file: '{}'...",path);
    let mut file = tokio::fs::File::create(&path).await?;
    file.write_all(&stream).await?;
    Ok(())
}

async fn test_post_api_response() -> HyperResult{
    let body = r#"{"code":202,"body":{}}"#;
    let response = Response::builder()
    .status(StatusCode::ACCEPTED)
    .body(Full::new(body.into()).map_err(|e| match e {}).boxed())
    .unwrap();
    Ok(response)
}


async fn post_api_response(req: Request<hyper::body::Incoming>) -> HyperResult{
    // Protect our server from massive bodies.
    let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
    if upper > 1024 * 1024 {
        return ServiceResponse::content_too_large()
    }
    
    let buffer = match req.collect().await{
      Ok(body) => body.aggregate(),
      Err(e) => {
        println!("{:?}",e);
        return ServiceResponse::bad_request()
      }  
    };
    let json_data = match RemoteResult::typed_json(buffer,&JSONKind::ProxyData){
        Ok(blob) => match blob.as_proxydata(){
            Ok(data) => data,
            Err(_) => return ServiceResponse::bad_request()
        },
        Err(e) => {
            println!("{:?}",e);
            return ServiceResponse::bad_request()
        }
    };
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type","application/json")
        .body(Full::new(json_data.into_bytes().into()).map_err(|e| match e {}).boxed())
        .unwrap();
    Ok(response)
}

