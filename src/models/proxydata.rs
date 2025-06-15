#![deny(warnings)]
use serde::{Deserialize,Serialize};
use crate::models::{RemoteData,RemoteResultType,RemoteResultInner,JSONKind,ResultKindError,JSONSerializeType};

#[derive(Serialize, Deserialize,Debug)]
pub struct ProxyData{
    data: String,
    target: String
}

impl ProxyData{
    pub fn into_bytes(self) -> Vec<u8>{
        self.data.into_bytes()
    }
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



impl RemoteResultInner for ProxyData{
    fn try_serialize(&self,_ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::ProxyData), data: self.data.clone().into()})
    }
}