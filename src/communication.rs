use std::string;
use std::thread;
use std::sync;
use std::path;
use std::error::Error;
use crc::{Crc, CRC_16_ARC};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub enum CSBIPacket {
    ACK,
    NACK,
    STOP,
    EOF,
    DATA(Vec<u8>)
}

impl CSBIPacket {
    /// This function constructs a byte array, containing the raw bytes that can be sent
    pub fn serialize(self) -> Vec<u8> {
        const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_ARC);
        match self {
            CSBIPacket::ACK => vec![0xd7],
            CSBIPacket::NACK => vec![0x27],
            CSBIPacket::STOP => vec![0xb4],
            CSBIPacket::EOF => vec![0x59],
            CSBIPacket::DATA(bytes) => {
                let mut v = vec![0x8bu8];
                let crc16 = CRC.checksum(&bytes);
                v.reserve_exact(4 + bytes.len());
                v.extend((bytes.len() as u16).to_be_bytes());
                v.extend(bytes);
                v.extend(crc16.to_be_bytes());
                v
            }
        }
    }
}

pub trait CommunicationHandle {
    /// Sends the bytes to the COBC, packaged accordingly
    fn send(&mut self, bytes: Vec<u8>) -> Result<()>;
    /// Blocks until a command from the COBC is received. Returns the raw bytes
    fn receive(&self) -> Result<Vec<u8>>;
    /// Sends the supplied packet
    fn send_packet(&mut self, p: CSBIPacket) -> Result<()> {
        self.send(p.serialize())
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
