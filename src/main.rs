use log;
use simplelog as sl;
use std::io::prelude::*;

mod communication;
mod command;
use command::Command;

fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
    
    let mut com = communication::spawn_communication_thread();

    loop {
        let received_file = com.receiver.recv().expect("Communication thread failed"); // Block until command is received
        let received_com = match command::process_payload(received_file) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Could not process command: {}", e);
                continue;
            }
        };
        
        let ret = match received_com {
            Command::StoreArchive(arch, bytes) => command::store_archive(&arch, bytes),
            _ => todo!()
        };
    }
}   
