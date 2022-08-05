use log;
use simplelog as sl;
use std::io::prelude::*;

mod communication;
mod command;
mod uart;
mod persist;
use crate::communication::{CSBIPacket, CommunicationHandle};
use command::CommandError;
use communication::CommunicationError;

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
                    com.send_packet(CSBIPacket::NACK).unwrap();
                },
                CommandError::CommunicationError(ce) => {
                    handle_communication_error(ce);
                },
                CommandError::InvalidCommError => {
                    log::error!("Received currently invalid command");
                    com.send_packet(CSBIPacket::NACK).unwrap();
                }
            };
        }
    }
}   

fn handle_communication_error(ce: CommunicationError) {
    match ce {
        CommunicationError::STOPCondition => {
            log::error!("Multi-packet communication stopped");
        },
        CommunicationError::InterfaceError => {
            log::error!("CommunicationHandle failed");
            panic!();
        },
        CommunicationError::PacketInvalidError => {
            log::error!("Received unknown packet");
        },
        CommunicationError::TimeoutError => {
            log::error!("Communication timed out");
        },            
        CommunicationError::CRCError => (),
    }
}
