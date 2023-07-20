use core::time;
use rppal::gpio::Gpio;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use simplelog as sl;

mod command;
mod communication;

#[derive(serde::Deserialize)]
struct Configuration {
    uart: String,
    baudrate: u32,
    heartbeat_pin: u8,
    update_pin: u8,
    heartbeat_freq: u64,
    log_path: String,
}

fn main() -> ! {
    let config: Configuration =
        toml::from_str(&std::fs::read_to_string("./config.toml").unwrap()).unwrap();

    // write all logging into a file
    let _ = sl::WriteLogger::init(
        sl::LevelFilter::Info,
        sl::Config::default(),
        std::fs::File::create(&config.log_path).unwrap(),
    );

    log::info!("Scheduler started");

    // construct a wrapper for UART communication
    let mut com = communication::UARTHandle::new(&config.uart, config.baudrate);

    // construct a wrapper for resources that are shared between different commands
    let ec = command::ExecutionContext::new("events".to_string(), config.update_pin).unwrap();
    let mut exec = Arc::new(Mutex::new(ec));

    // start a thread that will update the heartbeat pin
    thread::spawn(move || heartbeat_loop(config.heartbeat_pin, config.heartbeat_freq));

    // main loop
    loop {
        command::handle_command(&mut com, &mut exec);
    }
}

fn heartbeat_loop(heartbeat_pin: u8, freq: u64) -> ! {
    let toogle_time = time::Duration::from_millis((1000 / freq / 2) as u64);

    let gpio = Gpio::new().unwrap();
    let mut pin = gpio.get(heartbeat_pin).unwrap().into_output();

    loop {
        pin.set_high();
        thread::sleep(toogle_time);
        pin.set_low();
        thread::sleep(toogle_time);
    }
}
