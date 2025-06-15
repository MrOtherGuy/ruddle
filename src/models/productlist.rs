#![deny(warnings)]
use serde::{Deserialize,Serialize};
use crate::models::{RemoteData,RemoteResultType,RemoteResultInner,JSONKind,ResultKindError,JSONSerializeType};

#[derive(Serialize, Deserialize,Debug)]
pub struct ProductList{
    #[serde(rename = "ProductList")]
    product_list: Vec<RemoteProduct>
}

#[derive(Serialize, Deserialize,Debug)]
pub struct RemoteProduct{
    material_code: String,
    material_name: String,
    #[serde(rename = "EAN")]
    ean: String
}

impl ProductList{
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

impl RemoteResultInner for ProductList{
    fn try_serialize(&self,ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        match ser_type {
            JSONSerializeType::Pretty => match serde_json::to_vec_pretty(&self){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::ProductList), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            },
            JSONSerializeType::Dense => match serde_json::to_vec(&self){
                Ok(pretty_list) => Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::ProductList), data: pretty_list}),
                Err(e) => {
                    eprintln!("{}",e);
                    Err(ResultKindError::ConversionError)
                }
            }
        }
    }
}