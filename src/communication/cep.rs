use std::io::Read;

use crc::{Crc, CRC_32_MPEG_2};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CEPPacket {
    Ack,
    Nack,
    Stop,
    Eof,
    Data(Vec<u8>),
}

#[derive(Clone, Copy, strum::FromRepr)]
pub enum CEPPacketHeader {
    Ack = 0xd7,
    Nack = 0x27,
    Stop = 0xb4,
    Eof = 0x59,
    Data = 0x8b,
}

impl CEPPacket {
    pub const MAXIMUM_DATA_LENGTH: usize = 11000;
    pub const MAXIMUM_PACKET_LENGTH: usize = 7 + Self::MAXIMUM_DATA_LENGTH;

    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

    /// Calculates the CRC32 MPEG-2 checksum for the contained data. For variants other than Self::Data, 0 is returned
    pub fn checksum(&self) -> u32 {
        if let Self::Data(data) = self {
            Self::CRC.checksum(data)
        } else {
            0
        }
    }

    pub fn serialize(self) -> Vec<u8> {
        let header = self.header();
        match self {
            CEPPacket::Data(bytes) => {
                let mut v = Vec::with_capacity(7 + bytes.len());
                let crc32 = CEPPacket::CRC.checksum(&bytes);
                v.push(header);
                v.extend((bytes.len() as u16).to_le_bytes());
                v.extend(bytes);
                v.extend(crc32.to_le_bytes());
                v
            }
            _ => vec![header],
        }
    }

    pub fn crc_is_valid(data: &[u8], checksum: u32) -> bool {
        CEPPacket::CRC.checksum(data) == checksum
    }

    pub const fn header(&self) -> u8 {
        let header = match self {
            CEPPacket::Ack => CEPPacketHeader::Ack,
            CEPPacket::Nack => CEPPacketHeader::Nack,
            CEPPacket::Stop => CEPPacketHeader::Stop,
            CEPPacket::Eof => CEPPacketHeader::Eof,
            CEPPacket::Data(_) => CEPPacketHeader::Data,
        };
        header as u8
    }

    pub fn try_from_read(reader: &mut (impl Read + ?Sized)) -> Result<Self, CEPParseError> {
        let mut header_buffer = [0; 1];
        reader.read_exact(&mut header_buffer)?;

        let header = CEPPacketHeader::from_repr(header_buffer[0] as usize)
            .ok_or(CEPParseError::InvalidLength)?;
        let packet = match header {
            CEPPacketHeader::Ack => CEPPacket::Ack,
            CEPPacketHeader::Nack => CEPPacket::Nack,
            CEPPacketHeader::Stop => CEPPacket::Stop,
            CEPPacketHeader::Eof => CEPPacket::Eof,
            CEPPacketHeader::Data => {
                let mut length_buffer = [0; 2];
                reader.read_exact(&mut length_buffer)?;
                let length = u16::from_le_bytes(length_buffer);

                if length as usize > Self::MAXIMUM_DATA_LENGTH {
                    return Err(CEPParseError::InvalidLength);
                }

                let mut data_buffer = vec![0; length as usize];
                reader.read_exact(&mut data_buffer)?;

                let mut crc_buffer = [0; 4];
                reader.read_exact(&mut crc_buffer)?;
                if !CEPPacket::crc_is_valid(&data_buffer, u32::from_le_bytes(crc_buffer)) {
                    return Err(CEPParseError::InvalidCRC);
                }

                CEPPacket::Data(data_buffer)
            }
        };

        Ok(packet)
    }
}

impl From<&CEPPacket> for Vec<u8> {
    fn from(value: &CEPPacket) -> Self {
        match value {
            CEPPacket::Data(bytes) => {
                let mut v = Vec::with_capacity(7 + bytes.len());
                v.push(value.header());
                let crc32 = CEPPacket::CRC.checksum(bytes);
                v.extend((bytes.len() as u16).to_le_bytes());
                v.extend(bytes);
                v.extend(crc32.to_le_bytes());
                v
            }
            _ => vec![value.header()],
        }
    }
}

#[derive(Debug, strum::Display)]
pub enum CEPParseError {
    InvalidLength,
    InvalidHeader,
    InvalidCRC,
    Io(std::io::Error),
}

impl std::error::Error for CEPParseError {}

impl From<std::io::Error> for CEPParseError {
    fn from(value: std::io::Error) -> Self {
        CEPParseError::Io(value)
    }
}

impl TryFrom<Vec<u8>> for CEPPacket {
    type Error = CEPParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from_read(&mut std::io::Cursor::new(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(vec![0xD7], CEPPacket::Ack)]
    #[test_case(vec![0x27], CEPPacket::Nack)]
    #[test_case(vec![0x59], CEPPacket::Eof)]
    #[test_case(vec![0xB4], CEPPacket::Stop)]
    #[test_case(vec![0x8B, 0, 0, 0xff, 0xff, 0xff, 0xff], CEPPacket::Data(vec![]); "empty Data packet")]
    #[test_case(vec![0x8B, 4, 0, 0x0a, 0x0b, 0x05, 0x73, 0x52, 0x27, 0x92, 0xf4], CEPPacket::Data(vec![0x0a, 0x0b, 0x05, 0x73]); "filled data packet")]
    fn packet_is_parsed_and_serialized_correctly(vec: Vec<u8>, packet: CEPPacket) {
        assert_eq!(&packet.clone().serialize(), &vec);
        assert_eq!(CEPPacket::try_from(vec).unwrap(), packet);
    }

    #[test]
    fn invalid_crc_is_rejected() {
        assert!(
            matches!(
                CEPPacket::try_from(vec![0x8B, 4, 0, 0x0a, 0x0b, 0x05, 0x74, 0x52, 0x27, 0x92, 0xf4]),
                Err(CEPParseError::InvalidCRC)
            )
        )
    }

    #[test]
    fn invalid_length_is_rejected() {
        assert!(
            matches!(
                CEPPacket::try_from(vec![0x8B, 0xff, 0xff]),
                Err(CEPParseError::InvalidLength)
            )
        )
    }
}
