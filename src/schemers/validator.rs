#![deny(warnings)]
use serde_json::{Value};
use std::collections::{HashSet,HashMap};
use regex_lite::Regex;

#[allow(unused)]
#[derive(Debug)]
pub enum ValidationError{
    Invalid,
    NotBoolean,
    NotString,
    NotInt,
    NotFloat,
    NotArray,
    NotObject,
    NotNumber,
    OutOfRange,
    MissingRequired,
    UnexpectedProperty,
    PatternMatchFailed
}

#[allow(unused)]
#[derive(Debug)]
pub enum SchemaError{
    Invalid,
    MissingType,
    InvalidType,
    MissingProperties,
    InvalidProperties,
    MissingItems,
    InvalidItems,
    MissingProperty,
    InvalidProperty,
    MissingRequired,
    InvalidRequirements
}

impl std::fmt::Display for SchemaError{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            SchemaError::Invalid => write!(f, "Schema couldn't be parsed"),
            SchemaError::MissingProperty => write!(f, "Expected property"),
            SchemaError::MissingProperties => write!(f, "Expected property 'properties' is missing"),
            SchemaError::InvalidProperties => write!(f, "Value for property 'properties' is not valid"),
            SchemaError::MissingType => write!(f, "Expected property 'type' is missing"),
            SchemaError::InvalidType => write!(f, "Value for property 'type' is not valid"),
            SchemaError::MissingItems => write!(f, "Expected property 'items' is missing"),
            SchemaError::InvalidItems => write!(f, "Value for property 'items' is not valid"),
            SchemaError::InvalidProperty => write!(f, "Property has invalid value"),
            SchemaError::MissingRequired => write!(f, "Missing required property definition"),
            SchemaError::InvalidRequirements => write!(f, "Invalid property requirements")
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            ValidationError::Invalid => write!(f, "JSON doesn't match schema"),
            ValidationError::NotBoolean => write!(f, "Expected boolean"),
            ValidationError::NotNumber => write!(f, "Expected number"),
            ValidationError::NotArray => write!(f, "Expected array"),
            ValidationError::NotObject => write!(f, "Expected object"),
            ValidationError::NotString => write!(f, "Expected string"),
            ValidationError::OutOfRange => write!(f, "Property exists but is out of specified range"),
            ValidationError::NotInt => write!(f, "Expected integer"),
            ValidationError::NotFloat => write!(f, "Expected float"),
            ValidationError::MissingRequired => write!(f, "Missing required property"),
            ValidationError::UnexpectedProperty => write!(f, "Unexpected property"),
            ValidationError::PatternMatchFailed => write!(f, "Input doesn't match pattern")

        }
    }
}

#[derive(Debug)]
struct NumberNode{
    min: Option<f64>,
    max: Option<f64>
}

impl NumberNode{
    pub fn new() -> Self{
        Self{min: None, max: None}
    }
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError>{
        let n = match input.as_number(){
            Some(data) => data.as_f64().unwrap(),
            None => return Err(ValidationError::NotNumber)
        };
        if self.min.is_some() && n < self.min.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        if self.max.is_some() && n > self.max.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        Ok(())
    }
}
#[derive(Debug)]
struct FloatNode{
    min: Option<f64>,
    max: Option<f64>
}

impl FloatNode{
    pub fn new() -> Self{
        Self{min: None, max: None}
    }
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError>{
        let n = match input.as_f64(){
            Some(data) => data,
            None => return Err(ValidationError::NotFloat)
        };
        if self.min.is_some() && n < self.min.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        if self.max.is_some() && n > self.max.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        Ok(())
    }
}
#[derive(Debug)]
struct IntNode{
    min: Option<i64>,
    max: Option<i64>
}

impl IntNode{
    pub fn new() -> Self{
        Self{min: None, max: None}
    }
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError>{
        let n = match input.as_i64(){
            Some(data) => data,
            None => return Err(ValidationError::NotInt)
        };
        if self.min.is_some() && n < self.min.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        if self.max.is_some() && n > self.max.unwrap(){
            return Err(ValidationError::OutOfRange)
        }
        Ok(())
    }
}
#[derive(Debug)]
struct BoolNode{}

impl BoolNode{
    pub fn new() -> Self{
        Self{}
    }
    pub fn matches(&self,input: &Value) -> Result<(),ValidationError>{
        match input.is_boolean(){
            true => Ok(()),
            _ => Err(ValidationError::NotBoolean)
        }
    }
}
#[derive(Debug)]
struct StringNode{
    pattern: Option<Regex>
}

impl StringNode{
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError>{
        match input.as_str(){
            Some(data) => match &self.pattern{
                Some(re) => match re.is_match(data){
                    true => Ok(()),
                    false => Err(ValidationError::Invalid)
                },
                None => Ok(())
            },
            None => Err(ValidationError::NotString)
        }
    }
}
#[derive(Debug)]
struct ArrayNode{
    items: Box<PropertyKind>,
    length: Option<u16>,
}

impl ArrayNode{
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError>{
        match input.as_array() {
            Some(v) => {
                if let Some(length) = self.length {
                    if length as usize != v.len(){
                        return Err(ValidationError::OutOfRange)
                    }
                }
                match v.iter().all(|x| self.items.matches(x).is_ok()){
                    true => Ok(()),
                    false => Err(ValidationError::Invalid)
                }
            },
            None => Err(ValidationError::NotArray)
        }
    }
}
#[derive(Debug)]
enum PropertyKind{
    String(StringNode),
    Number(NumberNode),
    Float(FloatNode),
    Int(IntNode),
    Array(ArrayNode),
    Object(ObjectNode),
    Bool(BoolNode)
}

impl PropertyKind{
    fn matches(&self,input : &Value) -> Result<(),ValidationError>{
        match self {
            PropertyKind::String(s) => s.matches(input),
            PropertyKind::Number(s) => s.matches(input),
            PropertyKind::Float(s) => s.matches(input),
            PropertyKind::Int(s) => s.matches(input),
            PropertyKind::Array(s) => s.matches(input),
            PropertyKind::Object(s) => s.matches(input),
            PropertyKind::Bool(s) => s.matches(input)
        }
    }
}
#[derive(Debug)]
struct ObjectNode{
    properties: HashMap<String,PropertyKind>,
    required: HashSet<String>,
    allow_additional: bool
}

impl ObjectNode{
    pub fn matches(&self, input: &Value) -> Result<(),ValidationError> {
        match input.as_object(){
            Some(map) => {
                if !self.required.iter().all(|x| map.contains_key(x)){
                    println!("Missing property");
                    return Err(ValidationError::MissingRequired)
                }
                for (key,val) in map.iter(){
                    match (self.allow_additional, self.properties.get(key)){
                        (true, None) => continue,
                        (true, Some(v)) => if let Err(e) = v.matches(val){
                            eprintln!("{}, ({})",e,key);
                            return Err(ValidationError::Invalid)
                        },
                        (false, None) => {
                            eprintln!("Found unexpected property {}",key);
                            return Err(ValidationError::UnexpectedProperty)
                        },
                        (false, Some(v)) => if let Err(e) = v.matches(val){
                            eprintln!("{}, ({})",e,key);
                            return Err(ValidationError::Invalid)
                        }
                    }
                }
                Ok(())
            },
            None => Err(ValidationError::Invalid)
        }
    }
}
#[derive(Debug)]
enum ValidatorMode{
    AllPass,
    Normal
}
#[derive(Debug)]
pub struct Validator{
    schema: ObjectNode,
    mode: ValidatorMode
}

impl Validator{
    #[allow(unused)]
    pub fn transparent() -> Self{
        Validator{
            schema: ObjectNode{
                properties: HashMap::new(),
                required: HashSet::new(),
                allow_additional: true
            },
            mode: ValidatorMode::AllPass
        }
    }
    pub fn should_allow_all(&self) -> bool{
        match self.mode {
            ValidatorMode::AllPass => true,
            ValidatorMode::Normal => false
        }
    }
    pub fn from_json(input : Value) -> Result<Self,SchemaError> {
        match Validator::test_schema(&input) {
            Ok(kind) => match kind{
                PropertyKind::Object(s) => Ok(Validator{schema: s, mode: ValidatorMode::Normal}),
                _ => Err(SchemaError::Invalid)
            },
            Err(e) => Err(e)
        }
    }
    fn test_schema(input: &Value) -> Result<PropertyKind,SchemaError> {
        if !input.is_object(){
            return Err(SchemaError::Invalid)
        }
        let value = match input.get("type"){
            Some(v) => match v.as_str() {
                Some(s) => s,
                None => return Err(SchemaError::InvalidType)
            },
            None => return Err(SchemaError::MissingType)
        };
        match value{
            "array" => match Validator::test_validity_for_array(input){
                Ok(anode) => Ok(PropertyKind::Array(anode)),
                Err(e) => Err(e)
            },
            "object" => {
                let required: Option<Vec<&str>> = match input.get("required"){
                    Some(t) => match t.is_array(){
                        true => match t.as_array().unwrap().iter().all(|x| x.is_string()) {
                            true => Some(t.as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect()),
                            false => return Err(SchemaError::InvalidRequirements)
                        },
                        false => return Err(SchemaError::InvalidRequirements)
                    },
                    None => None
                };
                match Validator::test_validity_for_object(input,required){
                    Ok(onode) => Ok(PropertyKind::Object(onode)),
                    Err(e) => Err(e)
                }
            },
            "string" => {
                let pattern = match input.get("pattern"){
                    Some(t) => match t.as_str(){
                        Some(s) => match Regex::new(s){
                            Ok(r) => Some(r),
                            Err(e) => {
                                println!("{:?}",e);
                                return Err(SchemaError::Invalid)
                            }
                        },
                        None => return Err(SchemaError::Invalid)
                    },
                    None => None
                };
                Ok(PropertyKind::String(StringNode{ pattern: pattern }))
            },
            "boolean" => Ok(PropertyKind::Bool(BoolNode::new())),
            "number" => Ok(PropertyKind::Number(NumberNode::new())),
            "int" => Ok(PropertyKind::Int(IntNode::new())),
            "float" => Ok(PropertyKind::Float(FloatNode::new())),
            _ => return Err(SchemaError::InvalidType)
        }
    }
    fn test_validity_for_array(input : &Value) -> Result<ArrayNode,SchemaError> {
        let items = match input.get("items") {
            Some(v) => v,
            None => return Err(SchemaError::MissingItems)
        };
        if !items.is_object(){
            return Err(SchemaError::InvalidItems)
        }
        match Validator::test_schema(&items){
            Ok(s) => {
                let length = match input.get("length"){
                    Some(value) => match value.as_u64(){
                        Some(n) => match u16::try_from(n){
                            Ok(x) => Some(x),
                            Err(e) => {
                                println!("{:?}",e);
                                return Err(SchemaError::Invalid)
                            }
                        },
                        None => return Err(SchemaError::Invalid)
                    },
                    None => None
                };
                Ok(ArrayNode{items:Box::new(s),length: length})
            },
            Err(_) => Err(SchemaError::InvalidItems)
        }
    }
    fn test_validity_for_object(input : &Value,required: Option<Vec<&str>>) -> Result<ObjectNode,SchemaError> {
        let items = match input.get("properties") {
            Some(v) => v,
            None => return Err(SchemaError::MissingProperties)
        };
        if !items.is_object(){
            return Err(SchemaError::InvalidProperties)
        }
        
        let mut hashmap : HashMap<String,PropertyKind> = HashMap::new();
        let as_obj = items.as_object().unwrap();
        let mut hashset = HashSet::new();
        match required {
            Some(v) => {
                for prop in v{
                    match as_obj.get(prop) {
                        Some(r) => match Validator::test_schema(&r) {
                            Ok(kind) => {
                                hashmap.insert(prop.into(),kind);
                                hashset.insert(prop.into());
                            },
                            Err(e) => return Err(e)
                        },
                        None => {
                            println!("Missing required propery {}",prop);
                            return Err(SchemaError::MissingRequired)
                        }
                    }
                }
            },
            None => ()
        };
        
        for (key,val) in as_obj.iter(){
            if hashmap.contains_key(key){
                continue
            }
            match Validator::test_schema(&val) {
                Ok(kind) => hashmap.insert(key.into(),kind),
                Err(e) => return Err(e)
            };
        }
        let allow_additional = match input.get("additionalProperties"){
            Some(s) => s.as_bool().unwrap_or(false),
            None => false
        };

        Ok(ObjectNode{
            allow_additional: allow_additional,
            properties: hashmap,
            required: hashset
        })
    }
    pub fn validate(&self,target: &Value) -> Result<(),ValidationError> {
        
        if self.should_allow_all(){
            return Ok(())
        }
        println!("Validating schema...");
        match self.schema.matches(target){
            Ok(()) => {
                println!("Schema validation OK");
                Ok(())
            },
            Err(e) => {
                eprintln!("{}",e);
                Err(e)
            }
        }
    }

}