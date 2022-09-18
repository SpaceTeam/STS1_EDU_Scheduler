use crc::{Crc, CRC_32_MPEG_2};

#[derive(Debug, Clone, PartialEq)]
pub enum CSBIPacket {
    ACK,
    NACK,
    STOP,
    EOF,
    DATA(Vec<u8>),
}

impl CSBIPacket {
    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

    /// This function constructs a byte array, containing the raw bytes that can be sent
    pub fn serialize(self) -> Vec<u8> {
        match self {
            CSBIPacket::ACK => vec![0xd7],
            CSBIPacket::NACK => vec![0x27],
            CSBIPacket::STOP => vec![0xb4],
            CSBIPacket::EOF => vec![0x59],
            CSBIPacket::DATA(bytes) => {
                let mut v = vec![0x8b];
                let crc32 = CSBIPacket::CRC.checksum(&bytes);
                v.reserve_exact(6 + bytes.len());
                v.extend((bytes.len() as u16).to_be_bytes());
                v.extend(bytes);
                v.extend(crc32.to_be_bytes());
                v
            }
        }
    }

    pub fn check(data: &Vec<u8>, checksum: u32) -> bool {
        return CSBIPacket::CRC.checksum(data) == checksum;
    }
}
