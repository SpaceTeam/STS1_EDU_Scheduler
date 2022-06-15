use std::string;
use std::thread;
use std::sync;
use std::path;
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub trait CommunicationHandle {
    /// Sends the bytes to the COBC, packaged accordingly
    fn send(&mut self, bytes: Vec<u8>) -> Result<()>;
    /// Blocks until a command from the COBC is received. Returns the raw bytes
    fn receive(&self) -> Result<Vec<u8>>;
    /// Sends a NACK to the COBC
    fn send_nack(&mut self) -> Result<()> {
        return self.send(vec![0x33])
    }
    /// Sends a ACK to the COBC
    fn send_ack(&mut self) -> Result<()> {
        return self.send(vec![0x55])
    }
}

pub struct UARTHandle {
}

impl UARTHandle {
    pub fn new(baud: i32) -> UARTHandle {
        todo!();
    }
}

impl CommunicationHandle for UARTHandle {
    fn send(&mut self, bytes: Vec<u8>) -> Result<()> {
        todo!();
    }

    fn receive(&self) -> Result<Vec<u8>> {
        todo!();
    }
}
