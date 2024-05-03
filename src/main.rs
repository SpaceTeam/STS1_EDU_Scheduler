#![allow(non_snake_case)]
use core::time;
use rppal::gpio::Gpio;
use serialport::SerialPort;
use std::thread;
use STS1_EDU_Scheduler::communication::CommunicationHandle;

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
}

fn main() -> ! {
    let config: Configuration = toml::from_str(
        &std::fs::read_to_string("./config.toml").expect("Could not open config file"),
    )
    .unwrap();

    create_directory_if_not_exists("archives").unwrap();
    create_directory_if_not_exists("data").unwrap();

    // write all logging into a file
    let _ = sl::WriteLogger::init(
        sl::LevelFilter::Info,
        sl::Config::default(),
        std::fs::OpenOptions::new().create(true).append(true).open("log").unwrap(),
    );

    log::info!("Scheduler started");

    // construct a wrapper for UART communication
    let mut com =
        serialport::new(&config.uart, config.baudrate).open().expect("Could not open serial port");
    com.set_timeout(<Box<dyn SerialPort> as CommunicationHandle>::UNLIMITED_TIMEOUT);

    // construct a wrapper for resources that are shared between different commands
    let mut exec = command::ExecutionContext::new("events".to_string(), config.update_pin).unwrap();

    // start a thread that will update the heartbeat pin
    thread::spawn(move || heartbeat_loop(config.heartbeat_pin, config.heartbeat_freq));

    // main loop
    loop {
        command::handle_command(&mut com, &mut exec);
    }
}

fn heartbeat_loop(heartbeat_pin: u8, freq: u64) -> ! {
    if cfg!(feature = "mock") {
        std::thread::park();
    }

    let toogle_time = time::Duration::from_millis(1000 / freq / 2);

    let gpio = Gpio::new().unwrap();
    let mut pin = gpio.get(heartbeat_pin).unwrap().into_output();

    loop {
        pin.set_high();
        thread::sleep(toogle_time);
        pin.set_low();
        thread::sleep(toogle_time);
    }
}

/// Tries to create a directory, but only returns an error if the path does not already exists
fn create_directory_if_not_exists(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    match std::fs::create_dir(path) {
        Ok(_) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}
