#![deny(warnings)]
use std::fs::File;
use std::io::BufReader;
use std::collections::{HashSet,HashMap};
use super::validator::{SchemaError,Validator};
use serde_json::Value;

#[allow(unused)]
#[derive(Debug)]
pub struct SchemaTree{
    refs: HashMap<String,String>,
    schemas: HashMap<String,Validator>
}

impl SchemaTree{
    pub fn get_schema(&self,input: &str) -> Option<&Validator>{
        self.schemas.get(input)
    }
    pub fn contains_schema(&self,input: &str) -> bool{
        self.schemas.contains_key(input)
    }
    fn build_schemas_from_map(blob: &mut Value)-> Result<HashMap<String,Validator>,SchemaError>{
        match blob.as_object_mut(){
            Some(b) => {
                let mut temp_set : HashSet<String> = HashSet::new();
                for key in b.keys(){
                    temp_set.insert(key.clone());
                };
                let mut map: HashMap<String,Validator> = HashMap::new();
                for key in temp_set.iter(){
                    let t : Value = b.remove::<String>(key).unwrap();
                    match Validator::from_json(t){
                        Ok(v) => { map.insert(key.clone(),v); ()},
                        Err(_) => println!("Schema object with name '{}' is not valid",key)
                    }
                }
                Ok(map)
            },
            None => Err(SchemaError::Invalid)
        }
    }
    
    fn build_from_config(mut input: Value) -> Result<Self,SchemaError>{
        let schema_map = match input.as_object_mut(){
            Some(s) => match s.get_mut("schemas"){
                Some(mut schemas) => match SchemaTree::build_schemas_from_map(&mut schemas){
                    Ok(tree) => tree,
                    Err(_) => return Err(SchemaError::Invalid)
                },
                None => return Err(SchemaError::Invalid)
            },
            None => return Err(SchemaError::Invalid)
        };
        Ok(SchemaTree{
            schemas: schema_map,
            refs: HashMap::new()
        })
    }
    pub fn load_from_file(input: &str) -> Result<Self,SchemaError>{
        match SchemaTree::load_file(input) {
            Ok(json) => SchemaTree::build_from_config(json),
            Err(e) => {
                eprintln!("{}",e);
                Err(SchemaError::Invalid)
            }
        }
    }
    fn load_file(input: &str) -> Result<Value,std::io::Error>{
        let file = File::open(input)?;
        let reader = BufReader::new(file);
        let u : Value = serde_json::from_reader(reader)?;
        Ok(u)
    }
    
}

pub fn build_test() -> Result<SchemaTree,SchemaError>{
  let src = r#"{
"refs": {
},
"schemas": {
"test": {
"type": "object",
"required": [
  "RequiredTest"
],
"properties": {
  "RequiredTest": {
    "type": "array",
    "length": 2,
    "items": {
      "type": "object",
      "properties": {
        "test_code": {
          "type": "string",
          "pattern": "^h...o$"
        },
        "test_float": {
          "type": "float",
          "min": 2.5,
          "max": 5
        },
        "test_int": {
          "type": "int",
          "min": 0
        },
        "test_number": {
          "type": "number"
        }
      },
      "required": [
        "test_code",
        "test_float",
        "test_int"
      ],
      "additionalProperties": true
    }
    
  }
}
}
}
}"#;
  let v : Value = match serde_json::from_str(src){
      Ok(value) => value,
      Err(e) => {
          eprintln!("{}",e);
          return Err(SchemaError::Invalid)
      }
  };
  SchemaTree::build_from_config(v)
}