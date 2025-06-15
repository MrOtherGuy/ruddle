#![deny(warnings)]

pub mod productlist;
pub mod itemslist;
pub mod untyped;
pub mod proxydata;
pub mod untypedvalue;

use productlist::{ProductList};
use itemslist::{RemoteItemVec,RemoteItemsList};
use untyped::{RemoteUntyped,RemoteUntypedWrapper};
use proxydata::ProxyData;
use untypedvalue::UntypedValue;
use crate::schemers::validator::{ValidationError,Validator};

#[derive(Clone,Debug)]
pub enum JSONKind{
    ProductList,
    RemoteItem,
    Untyped,
    ProxyData,
    UntypedValue
}

pub enum JSONCompleteValue{
    ProductList(ProductList),
    RemoteItem(RemoteItemsList),
    Untyped(RemoteUntypedWrapper),
    Proxydata(ProxyData),
    UntypedValue(UntypedValue)
}

#[derive(Debug)]
pub enum ResultKindError{
    InvalidJSON,
    ConversionError,
    ValidationError(ValidationError)
}

impl std::fmt::Display for ResultKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            ResultKindError::InvalidJSON => write!(f, "Invalid JSON"),
            ResultKindError::ConversionError => write!(f, "Invalid call to serialize type"),
            ResultKindError::ValidationError(e) => write!(f, "Validation for json failed: {:?}",e)
        }
    }
}
#[derive(Debug,Clone)]
pub enum RemoteResultType{
    RemoteJSON(JSONKind),
    RemoteTXT,
    RemoteBytes
}

pub enum NativeDataKind{
    Condition,
    ProductData,
    Untyped,
    JSON,
    ProxyData,
    Text,
    Bytes
}

impl NativeDataKind{
    pub fn from_str(input : &str) -> Option<Self>{
        match input{
            "conditions" => Some(NativeDataKind::Condition),
            "productlist" => Some(NativeDataKind::ProductData),
            "untyped" => Some(NativeDataKind::Untyped),
            "json" => Some(NativeDataKind::JSON),
            "proxydata" => Some(NativeDataKind::ProxyData),
            "text" => Some(NativeDataKind::Text),
            "bytes" => Some(NativeDataKind::Bytes),
            _ => None
        }
    }
}

impl RemoteResultType{
    pub fn from_data_model(input : &NativeDataKind) -> Self{
        match input {
            NativeDataKind::Condition => RemoteResultType::RemoteJSON(JSONKind::RemoteItem),
            NativeDataKind::ProductData => RemoteResultType::RemoteJSON(JSONKind::ProductList),
            NativeDataKind::Untyped => RemoteResultType::RemoteJSON(JSONKind::Untyped),
            NativeDataKind::JSON => RemoteResultType::RemoteJSON(JSONKind::UntypedValue),
            NativeDataKind::ProxyData => RemoteResultType::RemoteJSON(JSONKind::ProxyData),
            NativeDataKind::Text => RemoteResultType::RemoteTXT,
            NativeDataKind::Bytes => RemoteResultType::RemoteBytes
        }
    }
    pub fn from_str(input : &str) -> Option<Self>{
        match NativeDataKind::from_str(input){
            None => None,
            Some(kind) => Some(RemoteResultType::from_data_model( &kind ))
        }
        
    }
    pub fn supports_schema(&self) -> bool{
        match self{
            RemoteResultType::RemoteJSON(JSONKind::UntypedValue) => true,
            _ => false
        }
    }
}

pub trait RemoteResultInner{
    fn try_serialize(&self,serializetype:&JSONSerializeType) -> Result<RemoteData,ResultKindError>;
}

impl RemoteResultInner for JSONCompleteValue{
    fn try_serialize(&self,ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        let res = match self{
            JSONCompleteValue::ProductList(list) => list.try_serialize(ser_type),
            JSONCompleteValue::RemoteItem(list) => list.try_serialize(ser_type),
            JSONCompleteValue::Untyped(list) => list.try_serialize(ser_type),
            JSONCompleteValue::Proxydata(list) => list.try_serialize(ser_type),
            JSONCompleteValue::UntypedValue(list) => list.try_serialize(ser_type)
        };
        match res{
            Ok(s) => Ok(s),
            Err(e) => Err(e)
        }
    }
}
#[allow(dead_code)]
impl JSONCompleteValue{
    pub fn as_productlist(self) -> Result<ProductList,ResultKindError>{
        match self{
            Self::ProductList(thing) => Ok(thing),
            _ => Err(ResultKindError::ConversionError)
        }
    }
    pub fn as_untyped(self) -> Result<RemoteUntyped,ResultKindError>{
        match self{
            Self::Untyped(thing) => Ok(thing.data),
            _ => Err(ResultKindError::ConversionError)
        }
    }
    pub fn as_remoteitems(self) -> Result<RemoteItemVec,ResultKindError>{
        match self{
            Self::RemoteItem(thing) => Ok(thing.items),
            _ => Err(ResultKindError::ConversionError)
        }
    }
    pub fn as_proxydata(self) -> Result<ProxyData,ResultKindError>{
        match self{
            Self::Proxydata(thing) => Ok(thing),
            _ => Err(ResultKindError::ConversionError)
        }
    }
    pub fn as_untypedvalue(self) -> Result<serde_json::Value,ResultKindError>{
        match self{
            Self::UntypedValue(thing) => Ok(thing.data),
            _ => Err(ResultKindError::ConversionError)
        }
    }
}

pub struct RemoteResult{}

#[derive(Debug,Clone)]
#[allow(unused)]
pub struct RemoteData{
    pub kind: RemoteResultType,
    data: Vec<u8>
}

impl RemoteData{
    pub fn data_bytes(&self) -> &Vec<u8>{
        &self.data
    }
    pub fn into_bytes(self) -> Vec<u8>{
        self.data
    }
}
#[allow(unused)]
pub enum JSONSerializeType{
    Pretty,
    Dense
}

impl RemoteResult{
    pub fn json(a_buf: impl hyper::body::Buf, kind: &JSONKind, serializetype: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        match RemoteResult::typed_json(a_buf, kind) {
            Ok(typed) => match typed.try_serialize(serializetype){
                Ok(remote) => Ok(remote),
                Err(_) => Err(ResultKindError::ConversionError)    
            },
            Err(_) => Err(ResultKindError::InvalidJSON)
        }
    }
    pub fn json_with_schema(a_buf: impl hyper::body::Buf, serializetype: &JSONSerializeType, validator: &Validator) -> Result<RemoteData,ResultKindError>{
        match RemoteResult::typed_json(a_buf, &JSONKind::UntypedValue) {
            Ok(typed) => {
                let untyped = typed.as_untypedvalue().unwrap();
                match validator.validate(&untyped) {
                    Ok(_) => match untyped.try_serialize(serializetype){
                        Ok(remote) => Ok(remote),
                        Err(_) => Err(ResultKindError::ConversionError)    
                    },
                    Err(e) => {
                        eprintln!("{}",e);
                        Err(ResultKindError::ValidationError(e))
                    }
                }
            },
            Err(_) => Err(ResultKindError::InvalidJSON)
        }
    }
    pub fn typed_json(a_buf: impl hyper::body::Buf, kind: &JSONKind ) -> Result<JSONCompleteValue,ResultKindError>{
        match kind{
            JSONKind::Untyped => {
                match RemoteUntypedWrapper::from_buffer(a_buf) {
                    Ok(list) => Ok(JSONCompleteValue::Untyped(list)),
                    Err(e) => Err(e)
                }
            },
            JSONKind::ProductList => {
                match ProductList::from_buffer(a_buf) {
                    Ok(list) => Ok(JSONCompleteValue::ProductList(list)),
                    Err(e) => Err(e)
                }
            },
            JSONKind::RemoteItem => {
                match RemoteItemsList::from_buffer(a_buf) {
                    Ok(list) => Ok(JSONCompleteValue::RemoteItem(list)),
                    Err(e) => Err(e)
                }
            },
            JSONKind::ProxyData => {
                match ProxyData::from_buffer(a_buf) {
                    Ok(list) => Ok(JSONCompleteValue::Proxydata(list)),
                    Err(e) => Err(e)
                }
            },
            JSONKind::UntypedValue => {
                match UntypedValue::from_buffer(a_buf) {
                    Ok(list) => Ok(JSONCompleteValue::UntypedValue(list)),
                    Err(e) => Err(e)
                }
            }
        }
    }
}