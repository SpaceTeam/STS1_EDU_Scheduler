#![allow(non_snake_case)]
use crate::command::Event;
use command::{ExecutionContext, RetryEvent};
use communication::socket::UnixSocketParser;
use core::time;
use rppal::gpio::Gpio;
use serialport::SerialPort;
use simplelog as sl;
use std::{
    io::ErrorKind,
    sync::{Arc, Mutex},
    thread,
};
use STS1_EDU_Scheduler::communication::CommunicationHandle;

mod command;
mod communication;

#[derive(serde::Deserialize)]
struct Configuration {
    uart: String,
    baudrate: u32,
    heartbeat_pin: u8,
    update_pin: u8,
    heartbeat_freq: u64,
    socket: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            uart: "/dev/serial0".to_string(),
            baudrate: 921_600,
            heartbeat_pin: 34,
            update_pin: 35,
            heartbeat_freq: 10,
            socket: "/tmp/scheduler_socket".to_string(),
        }
    }
}

fn main() -> ! {
    let _ = sl::WriteLogger::init(
        sl::LevelFilter::Info,
        sl::Config::default(),
        std::fs::OpenOptions::new().create(true).append(true).open("log").unwrap(),
    );

    let config: Configuration = if let Ok(s) = std::fs::read_to_string("./config.toml") {
        toml::from_str(&s)
            .map_err(|e| log::error!("Could not parse config {e:?}, using default"))
            .unwrap_or_default()
    } else {
        log::error!("Could not open config.toml, using default");
        Configuration::default()
    };

    create_directory_if_not_exists("archives").unwrap();
    create_directory_if_not_exists("data").unwrap();

    log::info!("Scheduler started");

    // construct a wrapper for UART communication
    let mut com =
        serialport::new(&config.uart, config.baudrate).open().expect("Could not open serial port");
    com.set_timeout(<Box<dyn SerialPort> as CommunicationHandle>::UNLIMITED_TIMEOUT);

    // construct a wrapper for resources that are shared between different commands
    let mut exec = command::ExecutionContext::new("events".to_string(), config.update_pin).unwrap();

    let socket_rx = communication::socket::UnixSocketParser::new(&config.socket).unwrap();
    let socket_context = exec.clone();
    std::thread::spawn(move || event_socket_loop(&socket_context, socket_rx));

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

fn event_socket_loop(context: &Arc<Mutex<ExecutionContext>>, mut socket: UnixSocketParser) {
    loop {
        let s = socket.read_object::<Event>();
        let event = match s {
            Ok(e) => e,
            Err(ref e) if e.kind() == ErrorKind::Other => break,
            Err(_) => continue,
        };

        log::info!("Received on socket: {event:?}");
        let mut context = context.lock().unwrap();
        context.event_vec.push(RetryEvent::new(event)).unwrap();
        context.configure_update_pin();
    }
}

/// Tries to create a directory, but only returns an error if the path does not already exists
fn create_directory_if_not_exists(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}
