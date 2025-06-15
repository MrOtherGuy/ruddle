#![deny(warnings)]

use std::collections::{HashSet};
use std::path::{Path,PathBuf};
use std::ffi::OsString;

#[derive(Debug,Clone)]
pub struct PathProvider<'a>{
    paths: PathSet,
    files: HashSet<&'a str>
}

impl PathProvider<'_>{
    pub fn contains_path(&self,test: &str) -> bool{
        match self.files.contains(test) {
            true => true,
            false => self.paths.contains_path(test)
        }
    }
    pub fn from_iter<'a>(values: impl Iterator<Item = config::Value>) -> Option<PathProvider<'a>>{
        let mut files : Vec<&str> = vec![];
        let mut dirs: Vec<PathBuf> = vec![];
        values.for_each(|k| (
            match k.into_string(){
                Ok(mut s) => match s.ends_with("/"){
                    true => { s.pop(); dirs.push(PathBuf::from(s).into_iter().fold(PathBuf::new(),|mut buf, x| { buf.push(x); return buf} )) },
                    false => files.push(["/",Path::new(s.as_str()).to_str().unwrap()].join("").leak())
                },
                Err(_) => ()
            }
        ));
        match (files.len(), files[0]){
            (1,"/*") => None,
            (_,_) => Some(PathProvider{
                files: HashSet::from_iter(files),
                paths: PathSet::from_paths(dirs)
            })
        }
    }
}

#[derive(Debug,Clone)]
pub struct PathSet{
    depth: usize,
    items: HashSet<OsString>
}

impl PathSet{
    pub fn contains_path(&self,a_path: &str) -> bool{
        let path = Path::new(a_path);
        let mut i = 0;
        let mut constructed = PathBuf::new();
        let mut comps = path.iter().skip(1);

        while let Some(p) = comps.next(){
            if i < self.depth{
                constructed.push(p);
                if self.items.contains(constructed.as_os_str()){
                    return true
                }
                i += 1
            }else{
                return false
            }
        }
        false
    }
    pub fn from_paths(v:Vec<PathBuf>) -> PathSet{
        let depth = v.iter()
        .map(|item| item.iter().collect::<Vec<_>>().len())
        .fold(0, |a,b| a.max(b));

        let vec : Vec<OsString> = v.into_iter().map(|x| x.into_os_string()).collect();
        PathSet{

            depth: depth,
            items: HashSet::from_iter(vec)
        }
    }
}