#![deny(warnings)]
use crate::models::{RemoteData,RemoteResultType,RemoteResultInner,JSONKind,ResultKindError,JSONSerializeType};

pub struct UntypedValue{
    pub data: serde_json::Value
}

impl UntypedValue{
    pub fn from_buffer(a_buf: impl hyper::body::Buf) -> Result<Self,ResultKindError>{
        let result: serde_json::Result<serde_json::Value> = serde_json::from_reader(a_buf.reader());
        
        match result {
            Ok(o) => Ok(UntypedValue{ data: o }),
            Err(_) => Err(ResultKindError::InvalidJSON)
        }
    }
}

impl RemoteResultInner for UntypedValue{
    fn try_serialize(&self,ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        match ser_type {
            JSONSerializeType::Pretty => match serde_json::to_vec_pretty(&self.data){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::UntypedValue), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            },
            JSONSerializeType::Dense => match serde_json::to_vec(&self.data){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::UntypedValue), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            }
        }
    }
}

impl RemoteResultInner for serde_json::Value{
    fn try_serialize(&self,ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        match ser_type {
            JSONSerializeType::Pretty => match serde_json::to_vec_pretty(&self){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::UntypedValue), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            },
            JSONSerializeType::Dense => match serde_json::to_vec(&self){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::UntypedValue), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            }
        }
    }
}