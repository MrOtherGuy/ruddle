#![deny(warnings)]
use clap::{Parser};

mod webview;

// build time generated block of static IMAGEDATA = [u8;40000]
include!(concat!(env!("OUT_DIR"), "/icon.rs"));

#[derive(Parser,Clone)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    silent: bool,
    #[arg(long)]
    url: String,
    #[arg(long)]
    width: u16,
    #[arg(long)]
    height: u16,
    #[arg(long)]
    title: String
}

pub fn main() -> () {
    let cli = Cli::parse();
    let v : Vec<u8> = Vec::from(IMAGEDATA);
    match webview::Webview::initialize(&cli.url, cli.width, cli.height, &cli.title,v){
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}",e);
        }
    }
}