use log;
use simplelog as sl;
use std::io::prelude::*;

mod communication;
mod command;
use command::Command;

fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
    
    let mut com = communication::UARTHandle::new(112500);
    let mut exec: Option<command::ExecutionContext> = None;

    loop {
        match command::process_command(&mut com, &mut exec) {
            Ok(()) => log::info!("Command executed successfully"),
            Err(e) => log::error!("Could not execute command <{}>", e)
        }
    }
}   
