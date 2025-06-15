#![deny(warnings)]
use serde::{Deserialize,Serialize};
use crate::models::{RemoteData,RemoteResultType,RemoteResultInner,JSONKind,ResultKindError,JSONSerializeType};

pub type RemoteItemVec = Vec<RemoteItem>;

#[derive(Serialize, Deserialize,Debug)]
pub struct RemoteItemsList{
    pub items: RemoteItemVec
}

#[derive(Serialize, Deserialize,Debug)]
pub struct RemoteItem{
    pub condition: String,
    pub effects: Vec<String>,
    pub data: Option<Vec<String>>
}

impl RemoteItemsList{
    pub fn from_buffer(a_buf: impl hyper::body::Buf) -> Result<Self,ResultKindError>{
        let result: serde_json::Result<Self> = serde_json::from_reader(a_buf.reader());
        match result {
            Ok(o) => Ok(o),
            Err(e) => {
                println!("{}",e);
                Err(ResultKindError::InvalidJSON)
            }
        }
    }
}

impl RemoteResultInner for RemoteItemsList{
    fn try_serialize(&self,ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        match ser_type {
            JSONSerializeType::Pretty => match serde_json::to_vec_pretty(&self.items){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::RemoteItem), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            },
            JSONSerializeType::Dense => match serde_json::to_vec(&self.items){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::RemoteItem), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            }
        }
    }
}