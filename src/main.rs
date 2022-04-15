use log;
use simplelog as sl;
use std::io::prelude::*;

mod command;



fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
}   
