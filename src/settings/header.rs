#![deny(warnings)]
use std::vec::Vec;


#[derive(Debug)]
pub enum HeaderError{
    ParseError,
    NotATable
}
impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            HeaderError::ParseError => write!(f, "Headers could not be parsed"),
            HeaderError::NotATable => write!(f, "Headers key is not a table"),
        }
    }
}
#[derive(Debug)]
pub(crate) struct HeaderSet{
    headers: Vec<Header>
}

pub(crate) enum ParseMode{
    IgnoreInvalid,
    Strict
}

impl HeaderSet{
    pub(crate) fn new() -> Self{
        Self{ headers: Vec::new() }
    }
    pub(crate) fn headers(&self) -> &Vec<Header>{
        &self.headers
    }
    #[allow(unused)]
    pub(crate) fn contains(&self, input: &str) -> bool{
        for item in &self.headers{
            if item.name.value == input{
                return true
            }
        }
        return false
    }
    #[allow(unused)]
    pub(crate) fn get_as_str(&self, input: &str) -> Option<&str>{
        for item in &self.headers{
            if item.name.value == input{
                return Some(item.value.to_value_str())
            }
        }
        return None
    } 
    pub(crate) fn parse(input: &config::Value,parsemode : ParseMode) -> Result<HeaderSet,HeaderError>{
        let table = match input.clone().into_table(){
            Ok(table) => table,
            Err(e) => {
                eprintln!("{e}");
                return Err(HeaderError::NotATable)
            }
        };
        let mut headers: Vec<Header> = Vec::new();
        for (key,val) in table.iter(){
            match val.clone().into_string(){
                Ok(s) => {
                    if s == "<auto>".to_string(){
                        headers.push(Header::new(key,HeaderValue::Computed("TODO".to_string())))
                    }else if s.starts_with("@"){
                        let mut copy = s.clone();
                        copy.remove(0);
                        headers.push(Header::new(key,HeaderValue::ByRequest(copy)))
                    }else{
                        headers.push(Header::new(key,HeaderValue::Literal(s.clone())))
                    }
                    ()
                },
                Err(e) => match parsemode{
                    ParseMode::Strict => {
                        eprintln!("{e}");
                        return Err(HeaderError::ParseError)
                    },
                    ParseMode::IgnoreInvalid => ()
                }
            };
        }
        Ok(HeaderSet{
            headers
        })
    }
    pub(crate) fn parse_literals(input: &config::Value,parsemode : ParseMode) -> Result<HeaderSet,HeaderError>{
        let table = match input.clone().into_table(){
            Ok(table) => table,
            Err(e) => {
                eprintln!("{e}");
                return Err(HeaderError::NotATable)
            }
        };
        let mut headers: Vec<Header> = Vec::new();
        for (key,val) in table.iter(){
            match val.clone().into_string(){
                Ok(s) => {
                    headers.push(Header::new(key,HeaderValue::Literal(s.clone())));
                    ()
                },
                Err(e) => match parsemode{
                    ParseMode::Strict => {
                        eprintln!("{e}");
                        return Err(HeaderError::ParseError)
                    },
                    ParseMode::IgnoreInvalid => ()
                }
            }
        };
        Ok(HeaderSet{
            headers
        })
    }
    
}

impl IntoIterator for HeaderSet{
    type Item = Header;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter{
        self.headers.into_iter()
    }
}
#[derive(Debug)]
pub(crate) struct Header{
    name: HeaderName,
    value: HeaderValue
}

impl Header{
    pub(crate) fn new(name: &str,value: HeaderValue) -> Self{
        Self{
            name: HeaderName{ value: name.to_string() },
            value: value
        }
    }
    pub fn name(&self) -> &HeaderName{
        &self.name
    }
    pub fn value(&self) -> &HeaderValue{
        &self.value
    }
    pub fn into_pair(self) -> (String,HeaderValue){
        (self.name.value, self.value)
    }
}
#[derive(Debug)]
pub(crate) struct HeaderName{
    value: String
}

impl HeaderName{
    pub fn as_str(&self) -> &str{
        self.value.as_str()
    }
}
#[derive(Debug,Clone)]
pub(crate) enum HeaderValue{
    Computed(String),
    Literal(String),
    ByRequest(String)
}

impl HeaderValue{
    pub fn to_value_str(&self) -> &str{
        match self{
            HeaderValue::Computed(s) => s.as_str(),
            HeaderValue::Literal(s) => s.as_str(),
            HeaderValue::ByRequest(s) => s.as_str() 
        }
    }
}
