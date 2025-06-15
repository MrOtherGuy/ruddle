#![deny(warnings)]
use std::future::{IntoFuture,ready,Ready};
use hyper::{Request,StatusCode,Response};
use crate::server_service::{HyperResult,ServerCommand,run_command,file_serve};
use http_body_util::{BodyExt, Full, Empty};
use crate::post_api::handle_post_api;


static NOTFOUND: &[u8] = b"Not Found";
static SERVICE_UNAVAILABLE: &[u8] = b"Service unavailable";
static BAD_METHOD: &[u8] = b"Method not allowed";

pub enum ServiceResponse{
    NotFound,
    NotFoundEmpty,
    ServiceUnavailable,
    PostAPIResponse,
    CommandResponse(ServerCommand),
    Accepted,
    FileService,
    BadMethod,
    BadRequest
}

impl IntoFuture for ServiceResponse{
    type Output = HyperResult;
    type IntoFuture = Ready<Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        match self{
            ServiceResponse::NotFound           => ready(ServiceResponse::not_found()),
            ServiceResponse::NotFoundEmpty      => ready(ServiceResponse::not_found_empty()),
            ServiceResponse::ServiceUnavailable => ready(ServiceResponse::service_unavailable()),
            ServiceResponse::PostAPIResponse => panic!("PostAPIResponse should not get called"),
            ServiceResponse::CommandResponse(_) => panic!("CommandResponse should not get called"),
            ServiceResponse::Accepted           => ready(ServiceResponse::accepted()),
            ServiceResponse::FileService     => panic!("FileService should not get called"),
            ServiceResponse::BadMethod          => ready(ServiceResponse::bad_method()),
            ServiceResponse::BadRequest         => ready(ServiceResponse::bad_request())
        }
    }
}

impl ServiceResponse{
    pub async fn resolve(self,request:Request<hyper::body::Incoming>) -> HyperResult{
        match self{
            ServiceResponse::FileService => file_serve(request).await,
            ServiceResponse::CommandResponse(command) => run_command(&command,request.headers()).await,
            ServiceResponse::PostAPIResponse => handle_post_api(request).await,
            _ => self.await
        }
    }
    pub fn bad_method() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(Full::new(BAD_METHOD.into()).map_err(|e| match e {}).boxed())
        .unwrap())
    }
    
    pub fn service_unavailable() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(Full::new(SERVICE_UNAVAILABLE.into()).map_err(|e| match e {}).boxed())
        .unwrap())
    }

    pub fn not_acceptable() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::NOT_ACCEPTABLE)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
    }
    
    pub fn accepted() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
    }

    pub fn created() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::CREATED)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
    }
    
    pub fn bad_request() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
    }

    pub fn content_too_large() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::PAYLOAD_TOO_LARGE)
        .body(Full::new("Content Too Large".into()).map_err(|e| match e {}).boxed())
        .unwrap())
    }

    /// HTTP status code 404
    pub fn not_found() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOTFOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap())
    }
    pub fn not_found_empty() -> HyperResult {
        Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
    }
}