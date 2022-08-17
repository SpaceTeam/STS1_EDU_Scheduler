use core::time;
use std::{thread, sync::{Arc, Mutex}};
use rppal::gpio::Gpio;

use log;
use simplelog as sl;

mod communication;
mod command;
mod persist;
use command::CommandError;
use communication::CommunicationError;


fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
    
    let mut com = communication::UARTHandle::new(112500);
    let mut ec = command::ExecutionContext::new("./data/status_queue".into(), "./data/result_queue".into()).unwrap();
    let mut exec = Arc::new(Mutex::new(ec));

    //Heartbeat thread
    thread::spawn(|| {
        const HEARTBEAT_FREQ: u64 = 2;
        const HEARTBEAT_PIN: u8 = 23;
        const TOGGLE_TIME_MS: time::Duration = time::Duration::from_millis(HEARTBEAT_FREQ * 500);

        let gpio = Gpio::new().unwrap();
        let mut pin = gpio.get(HEARTBEAT_PIN).unwrap().into_output();

        loop {
            pin.set_high();
            thread::sleep(TOGGLE_TIME_MS);
            pin.set_low();
            thread::sleep(TOGGLE_TIME_MS);
        }
    });

    loop {
        let ret = command::handle_command(&mut com, &mut exec);
        
        if let Err(e) = ret {
            match e {
                CommandError::SystemError(ioe) => {
                    log::error!("Command failed with {}", ioe);
                },
                CommandError::CommunicationError(ce) => {
                    handle_communication_error(ce);
                },
                CommandError::InvalidCommError => {
                    log::error!("Received currently invalid command");
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
