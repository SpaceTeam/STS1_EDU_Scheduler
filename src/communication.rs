use std::string;
use std::thread;
use std::sync;
use std::path;
use std::error::Error;

pub trait CommunicationHandle {
    /// Sends the bytes to the COBC, packaged accordingly
    fn send(&mut self, bytes: Vec<u8>);
    /// Blocks until a command from the COBC is received. Returns the raw bytes
    fn receive(&self) -> Result<Vec<u8>, Box<dyn Error>>;
    /// Sends a NACK to the COBC
    fn send_nack(&self);
    /// Sends a ACK to the COBC
    fn send_ack(&self);
}

pub struct UARTHandle {
}

impl UARTHandle {
    pub fn new(baud: i32) -> UARTHandle {
        todo!();
    }
}

impl CommunicationHandle for UARTHandle {
    fn send(&mut self, bytes: Vec<u8>) {
        todo!();
    }

    fn receive(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        todo!();
    }

    fn send_ack(&self) {
        todo!();
    }

    fn send_nack(&self) {
        todo!();
    }
}
