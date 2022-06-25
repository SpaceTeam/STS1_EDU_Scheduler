use log;
use simplelog as sl;
use std::io::prelude::*;

mod communication;
mod command;
mod uart;
use crate::communication::{CSBIPacket, CommunicationHandle};
use command::CommandError;

fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
    
    let mut com = uart::UARTHandle::new(112500);
    let mut exec: Option<command::ExecutionContext> = None;

    loop {
        let ret = command::process_command(&mut com, &mut exec);
        if let Err(e) = ret {
            match e {
                CommandError::SystemError(ioe) => {
                    log::error!("Command failed with {}", ioe);
                    com.send_packet(CSBIPacket::NACK);
                },
                CommandError::ComError => {
                    log::error!("Received invalid data");
                    com.send_packet(CSBIPacket::NACK);
                },
                CommandError::InterfaceError => {
                    log::error!("Could not send or receive data");
                    panic!("Communication module failed");
                },
                CommandError::Interrupted => {
                    log::info!("Command was interrupted");
                }
            };
        }
    }
}   
