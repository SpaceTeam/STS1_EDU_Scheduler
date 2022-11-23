use core::time;
use rppal::gpio::Gpio;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use log;
use simplelog as sl;

mod command;
mod communication;
mod persist;
use command::CommandError;
use communication::CommunicationError;

fn main() {
    let _ = sl::WriteLogger::init(
        sl::LevelFilter::Info,
        sl::Config::default(),
        std::fs::File::create("log").unwrap(),
    );

    const UPDATE_PIN: u8 = 35;

    let mut com = communication::UARTHandle::new(921600);
    let ec = command::ExecutionContext::new(
        "./data/status_queue".into(),
        "./data/result_queue".into(),
        UPDATE_PIN,
    )
    .unwrap();
    let mut exec = Arc::new(Mutex::new(ec));

    //Heartbeat thread
    thread::spawn(|| {
        const HEARTBEAT_FREQ: u64 = 10; //Hz
        const HEARTBEAT_PIN: u8 = 34;
        const TOGGLE_TIME_MS: time::Duration =
            time::Duration::from_millis((1000 / HEARTBEAT_FREQ / 2) as u64);

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
                }
                CommandError::CommunicationError(ce) => {
                    handle_communication_error(ce);
                }
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
        }
        CommunicationError::InterfaceError => {
            log::error!("CommunicationHandle failed");
            panic!();
        }
        CommunicationError::PacketInvalidError => {
            log::error!("Received unknown packet");
        }
        CommunicationError::TimeoutError => {
            log::error!("Communication timed out");
        }
        CommunicationError::CRCError => (),
    }
}
