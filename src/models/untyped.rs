#![deny(warnings)]
use serde::{Deserialize};
use std::io::Read;
use crate::models::{RemoteData,RemoteResultType,RemoteResultInner,JSONKind,ResultKindError,JSONSerializeType};

#[allow(unused)]
pub type RemoteUntyped = Vec<u8>;

#[derive(Deserialize,Debug)]
pub struct RemoteUntypedWrapper{
    pub data: Vec<u8>
}

impl RemoteUntypedWrapper{
    pub fn from_buffer(a_buf: impl hyper::body::Buf) -> Result<Self,ResultKindError>{
        let mut reader = a_buf.reader();
        let mut m_vec : Vec<u8> = vec![];
        match reader.read_to_end(&mut m_vec) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}",e);
                return Err(ResultKindError::ConversionError)
            }
        };
        let contents = match String::from_utf8(m_vec){
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}",e);
                return Err(ResultKindError::ConversionError)
            }
        };
        let c = config::Config::builder()
        .add_source(config::File::from_str(&contents, config::FileFormat::Json))
        .build();
        match c {
            Ok(_) => Ok(RemoteUntypedWrapper{data: contents.into_bytes()}),
            Err(e) => {
                eprintln!("{}",e);
                Err(ResultKindError::InvalidJSON)
            }
        }
    }
}

impl RemoteResultInner for RemoteUntypedWrapper{
    fn try_serialize(&self,_ser_type: &JSONSerializeType) -> Result<RemoteData,ResultKindError>{
        Ok(RemoteData{kind: RemoteResultType::RemoteJSON(JSONKind::Untyped), data: self.data.clone()})
    }
}