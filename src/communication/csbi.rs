use crc::{Crc, CRC_32_MPEG_2};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CEPPacket {
    ACK,
    NACK,
    STOP,
    EOF,
    DATA(Vec<u8>),
}

impl CEPPacket {
    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

    /// This function constructs a byte array, containing the raw bytes that can be sent
    pub fn serialize(self) -> Vec<u8> {
        match self {
            CEPPacket::ACK => vec![0xd7],
            CEPPacket::NACK => vec![0x27],
            CEPPacket::STOP => vec![0xb4],
            CEPPacket::EOF => vec![0x59],
            CEPPacket::DATA(bytes) => {
                let mut v = vec![0x8b];
                let crc32 = CEPPacket::CRC.checksum(&bytes);
                v.reserve_exact(6 + bytes.len());
                v.extend((bytes.len() as u16).to_le_bytes());
                v.extend(bytes);
                v.extend(crc32.to_le_bytes());
                v
            }
        }
    }

    pub fn check(data: &Vec<u8>, checksum: u32) -> bool {
        CEPPacket::CRC.checksum(data) == checksum
    }
}
