use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>>{
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("icon.rs");
    let data: Vec<u8> = fs::read("icon_data/icon.dat")?;
    let content = String::from_utf8(data)?;
    let mut string = String::with_capacity(85208);
    string.push_str("static IMAGEDATA: [u8;40000] = [");
    string.push_str(&content);
    string.push_str("];");
    fs::write(
        &dest_path,string
        
    ).unwrap();
    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}

