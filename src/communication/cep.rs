use crc::{Crc, CRC_32_MPEG_2};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CEPPacket {
    ACK,
    NACK,
    STOP,
    EOF,
    DATA(Vec<u8>),
}

#[derive(Clone, Copy, strum::FromRepr)]
pub enum CEPPacketHeader {
    ACK = 0xd7,
    NACK = 0x27,
    STOP = 0xb4,
    EOF = 0x59,
    DATA = 0x8b,
}

impl CEPPacket {
    pub const MAXIMUM_DATA_LENGTH: usize = 32768;
    pub const MAXIMUM_PACKET_LENGTH: usize = 7 + Self::MAXIMUM_DATA_LENGTH;

    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

    /// Calculates the CRC32 MPEG-2 checksum for the contained data. For variants other than Self::DATA, 0 is returned
    pub fn checksum(&self) -> u32 {
        if let Self::DATA(data) = self {
            Self::CRC.checksum(&data)
        } else {
            0
        }
    }

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

    pub fn crc_is_valid(data: &[u8], checksum: u32) -> bool {
        CEPPacket::CRC.checksum(data) == checksum
    }

    pub const fn header(&self) -> u8 {
        let header = match self {
            CEPPacket::ACK => CEPPacketHeader::ACK,
            CEPPacket::NACK => CEPPacketHeader::NACK,
            CEPPacket::STOP => CEPPacketHeader::STOP,
            CEPPacket::EOF => CEPPacketHeader::EOF,
            CEPPacket::DATA(_) => CEPPacketHeader::DATA,
        };
        header as u8
    }
}

impl From<&CEPPacket> for Vec<u8> {
    fn from(value: &CEPPacket) -> Self {
        match value {
            CEPPacket::DATA(bytes) => {
                let mut v = Vec::with_capacity(7 + bytes.len());
                v.push(value.header());
                let crc32 = CEPPacket::CRC.checksum(&bytes);
                v.extend((bytes.len() as u16).to_le_bytes());
                v.extend(bytes);
                v.extend(crc32.to_le_bytes());
                v
            }
            _ => vec![value.header()],
        }
    }
}

#[derive(Debug)]
pub enum CEPParseError {
    WrongLength,
    InvalidHeader,
    InvalidCRC,
}

impl TryFrom<Vec<u8>> for CEPPacket {
    type Error = CEPParseError;

    fn try_from(mut value: Vec<u8>) -> Result<Self, Self::Error> {
        let header_byte = value.get(0).ok_or(CEPParseError::WrongLength)?;
        let header = CEPPacketHeader::from_repr(*header_byte as usize)
            .ok_or(CEPParseError::InvalidHeader)?;

        let packet = match header {
            CEPPacketHeader::ACK => CEPPacket::ACK,
            CEPPacketHeader::NACK => CEPPacket::NACK,
            CEPPacketHeader::STOP => CEPPacket::STOP,
            CEPPacketHeader::EOF => CEPPacket::EOF,
            CEPPacketHeader::DATA => {
                let length_bytes = value.get(1..3).ok_or(CEPParseError::WrongLength)?;
                let length = u16::from_le_bytes(length_bytes.try_into().unwrap()) as usize;
                value.drain(0..3);

                let crc_bytes = value.drain(length..length + 4);
                let crc = u32::from_le_bytes(crc_bytes.as_slice().try_into().unwrap());
                drop(crc_bytes);

                if !CEPPacket::crc_is_valid(&value, crc) {
                    return Err(CEPParseError::InvalidCRC);
                }

                CEPPacket::DATA(value)
            }
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(vec![0xD7], CEPPacket::ACK)]
    #[test_case(vec![0x27], CEPPacket::NACK)]
    #[test_case(vec![0x59], CEPPacket::EOF)]
    #[test_case(vec![0xB4], CEPPacket::STOP)]
    #[test_case(vec![0x8B, 0, 0, 0xff, 0xff, 0xff, 0xff], CEPPacket::DATA(vec![]); "empty DATA packet")]
    fn packet_is_parsed_and_serialized_correctly(vec: Vec<u8>, packet: CEPPacket) {
        assert_eq!(&packet.clone().serialize(), &vec);
        assert_eq!(CEPPacket::try_from(vec).unwrap(), packet);
    }
}
