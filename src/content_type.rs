#![deny(warnings)]
use std::collections::HashMap;
use hyper::Response;
use crate::Settings;
use crate::settings::HeaderValue;
// Badly named, but this is response headers only
pub(crate) type HeaderMap = HashMap<ContentType,HashMap<String,HeaderValue>>;
pub struct MIMEParseError{}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ContentType{
    Javascript,
    Json,
    CSS,
    HTML,
    PlainText,
    ImageJPG,
    ImagePNG,
    ImageSVG,
    ImageICO,
    Unknown,
    Global,
    Wasm,
    Wat
}

#[derive(Debug,Clone)]
pub enum NegotiationError{
    NotAcceptable
}

impl std::fmt::Display for NegotiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            NegotiationError::NotAcceptable => write!(f, "Couldn't find fitting content-type for client"),
        }
    }
}

pub type NegotiationResult = Result<hyper::http::response::Builder,NegotiationError>;

impl std::str::FromStr for ContentType{
    type Err = MIMEParseError;
    fn from_str(s: &str) -> Result<Self, MIMEParseError> {
        match s.rfind("."){
            Some(start) => match s.get(start..){
                Some(rest) => {
                    Ok(match rest{
                        ".js" | ".mjs" => ContentType::Javascript,
                        ".json" => ContentType::Json,
                        ".css" => ContentType::CSS,
                        ".html" | ".htm" => ContentType::HTML,
                        ".txt" => ContentType::PlainText,
                        ".jpg" | ".jpeg" => ContentType::ImageJPG,
                        ".png" | ".apng" =>  ContentType::ImagePNG,
                        ".svg" => ContentType::ImageSVG,
                        ".ico" => ContentType::ImageICO,
                        ".wasm" => ContentType::Wasm,
                        ".wat" => ContentType::Wat,
                        _ => ContentType::Unknown
                    })
                },
                _ => Ok(ContentType::Unknown)
            },
            _ => Ok(ContentType::Unknown)
        }
    }
}

impl std::fmt::Display for MIMEParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MIME type parsing failed")
    }
}

pub trait GetHeaderValueString{
    fn get_as_string(&self,header_name: &str) -> Option<&str>;
}

impl GetHeaderValueString for hyper::HeaderMap{
    fn get_as_string(&self, header_name: &str) -> Option<&str>{
        match self.get(header_name){
            Some(hv) => match hv.to_str(){
                Ok(value) => Some(value),
                Err(e) => {eprintln!("{e}");None}
            },
            None => None
        }
    }
}

impl ContentType{
    pub fn into_response(self, config: &Settings, headers: &hyper::HeaderMap) -> NegotiationResult{
        let content_type = self.get_content_type_if_supported(headers);
        if content_type.is_none(){
            return Err(NegotiationError::NotAcceptable)
        }
        let mut builder = Response::builder().header("Content-Type",self.to_str());
        let header_map = match config.header_map.get(&self) {
            Some(headers) => Some(headers),
            None => config.header_map.get(&ContentType::Global)
        };
        if let Some(response_headers) = header_map{
            for (key,header_val) in response_headers.iter(){
                match header_val{
                    HeaderValue::Literal(val) => {
                        builder = builder.header(key.clone(), val.clone());
                    },
                    HeaderValue::Computed(val) => {
                        builder = builder.header(key.clone(), val.clone());
                    },
                    HeaderValue::ByRequest(val) => match headers.get_as_string(val){
                        Some(hv) => {
                            if config.allow_origins.contains(hv){
                                builder = builder.header(key.clone(), hv);
                            }
                        },
                        None => ()
                    }
                };
            }
        }
        Ok(builder)
    }
    pub fn get_content_type_if_supported(&self, headers: &hyper::HeaderMap) -> Option<&str>{
        if let Some(request_accept) = headers.get_as_string("Accept"){
            let target_value = self.to_str();
            let media_type = self.media_type();
            for part in request_accept.split(","){
                let slice : &str = part.split(";").next().unwrap();
                if slice.starts_with("*/*") || slice == target_value{
                    return Some(target_value)
                }
                match slice.split_once("/"){
                    Some((left,right)) => match right{
                        "*" => if left == media_type{
                            return Some(target_value)
                        },
                        _ => ()
                        
                    },
                    None => ()
                }
            }
            return None
        }
        Some(self.to_str())
    }
    pub fn media_type(&self) -> &str{
        match self{
            ContentType::Javascript => "application",
            ContentType::Json => "application",
            ContentType::CSS => "text",
            ContentType::HTML => "text",
            ContentType::PlainText => "text",
            ContentType::ImageJPG => "image",
            ContentType::ImagePNG => "image",
            ContentType::ImageSVG => "image",
            ContentType::ImageICO => "image",
            ContentType::Unknown =>  "application",
            ContentType::Wasm => "application",
            ContentType::Wat => "application",
            ContentType::Global => panic!("Global content type must not be stringified!")
        }
    }
    pub fn from_mime_type(s: &str) -> ContentType{
        match s{
            "application/javascript" => ContentType::Javascript,
            "application/json" => ContentType::Json,
            "application/wasm" => ContentType::Wasm,
            "application/wat" => ContentType::Wat,
            "text/css" => ContentType::CSS,
            "text/html" => ContentType::HTML,
            "text/plain" => ContentType::PlainText,
            "image/jpg" => ContentType::ImageJPG,
            "image/png" => ContentType::ImagePNG,
            "image/svg+xml" => ContentType::ImageSVG,
            "image/x-icon" => ContentType::ImageICO,
            "Global" => ContentType::Global,
            _ => ContentType::Unknown
        }
    }
    fn to_str(&self) -> &str {
        match self {
            ContentType::Javascript => "application/javascript",
            ContentType::Json => "application/json",
            ContentType::Wasm => "application/wasm",
            ContentType::Wat => "application/wat",
            ContentType::CSS => "text/css",
            ContentType::HTML => "text/html",
            ContentType::PlainText => "text/plain",
            ContentType::ImageJPG => "image/jpg",
            ContentType::ImagePNG => "image/png",
            ContentType::ImageSVG => "image/svg+xml",
            ContentType::ImageICO => "image/x-icon",
            ContentType::Unknown =>  "application/octet-stream",
            ContentType::Global => panic!("Global content type must not be stringified!")
        }
    }
}