use crc::{Crc, CRC_16_ARC};
use std::error::Error;


type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub enum CSBIPacket {
    ACK,
    NACK,
    STOP,
    EOF,
    DATA(Vec<u8>),
    INVALID
}

impl CSBIPacket {
    const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_ARC);

    /// This function constructs a byte array, containing the raw bytes that can be sent
    pub fn serialize(self) -> Vec<u8> {
        match self {
            CSBIPacket::ACK => vec![0xd7],
            CSBIPacket::NACK => vec![0x27],
            CSBIPacket::STOP => vec![0xb4],
            CSBIPacket::EOF => vec![0x59],
            CSBIPacket::DATA(bytes) => {
                let mut v = vec![0x8b];
                let crc16 = CSBIPacket::CRC.checksum(&bytes);
                v.reserve_exact(4 + bytes.len());
                v.extend((bytes.len() as u16).to_be_bytes());
                v.extend(bytes);
                v.extend(crc16.to_be_bytes());
                v
            },
            _ => vec![0x00]
        }
    }

    pub fn check(data: &Vec<u8>, checksum: u16) -> bool {
        return CSBIPacket::CRC.checksum(data) == checksum;
    }
}

pub trait CommunicationHandle {
    /// Sends the bytes to the COBC, packaged accordingly
    fn send(&mut self, bytes: Vec<u8>) -> Result<()>;

    /// Blocks until n bytes are received or the timeout is reached
    fn receive(&self, n: u16) -> Result<Vec<u8>>;

    /// Sends the supplied packet
    fn send_packet(&mut self, p: CSBIPacket) -> Result<()> {
        self.send(p.serialize())
    }

    /// Blocks until it receives a CSBIPacket
    fn receive_packet(&self) -> Result<CSBIPacket> {
        let p = match self.receive(1)?[0] {
            0xd7 => CSBIPacket::ACK,
            0x27 => CSBIPacket::NACK,
            0xb4 => CSBIPacket::STOP,
            0x59 => CSBIPacket::EOF,
            0xb8 => {
                let length_field = self.receive(2)?;
                let length = u16::from_be_bytes([length_field[0], length_field[1]]);
                let bytes = self.receive(length)?;
                let crc_field = self.receive(2)?;
                let crc = u16::from_be_bytes([crc_field[0], crc_field[1]]);
                if !CSBIPacket::check(&bytes, crc) {
                    CSBIPacket::INVALID
                }
                else {
                    CSBIPacket::DATA(bytes)
                }
            }
            _ => {
                return Err("Invalid header".into());
            }
        };

        Ok(p)
    }

    /// Attempts to continously receive multidata packets and returns them in a concatenated byte vector
    /// `stop_fn` is evaluated after every packet and terminates the communication with a STOP packet if true
    /// An error is returned in this case
    fn receive_multi_packet(&mut self, stop_fn: impl Fn() -> bool) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        loop {
            let pack = self.receive_packet()?;
            if stop_fn() {
                self.send_packet(CSBIPacket::STOP)?;
                return Err("Communication stopped from self".into());
            }

            match pack {
                CSBIPacket::DATA(b) => {
                    buffer.extend(b);
                    self.send_packet(CSBIPacket::ACK)?;
                },
                CSBIPacket::EOF => {
                    break;
                },
                CSBIPacket::STOP => {
                    return Err("Communication stopped".into());
                },
                _ => {
                    self.send_packet(CSBIPacket::NACK)?;
                }
            };
        }

        return Ok(buffer);
    }
}

pub struct UARTHandle {}

impl UARTHandle {
    pub fn new(baud: i32) -> UARTHandle {
        todo!();
    }
}

impl CommunicationHandle for UARTHandle {
    fn send(&mut self, bytes: Vec<u8>) -> Result<()> {
        todo!();
    }

    fn receive(&self, n: u16) -> Result<Vec<u8>> {
        todo!();
    }
}
