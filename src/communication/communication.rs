use std::{
    io::{Read, Write},
    time::Duration,
};

use super::{cep::CEPPacketHeader, CEPPacket};

pub type ComResult<T> = Result<T, CommunicationError>;

pub trait CommunicationHandle: Read + Write {
    const INTEGRITY_ACK_TIMEOUT: Duration;
    const UNLIMITED_TIMEOUT: Duration;

    fn set_timeout(&mut self, timeout: &Duration);

    fn send_packet(&mut self, packet: &CEPPacket) -> ComResult<()> {
        self.write_all(&[packet.header()])?;

        if let CEPPacket::DATA(data) = packet {
            self.write_all(&(data.len() as u16).to_le_bytes())?;
            self.write_all(&data)?;
            self.write_all(&packet.checksum().to_le_bytes())?;
            self.flush()?;

            self.await_ack(&Self::INTEGRITY_ACK_TIMEOUT)?;
        }

        Ok(())
    }

    fn send_multi_packet(&mut self, bytes: &[u8]) -> ComResult<()> {
        let chunks = bytes.chunks(CEPPacket::MAXIMUM_DATA_LENGTH);
        for chunk in chunks {
            self.send_packet(&CEPPacket::DATA(chunk.into()))?;
        }

        self.send_packet(&CEPPacket::EOF)?;
        self.await_ack(&Self::INTEGRITY_ACK_TIMEOUT)?;

        Ok(())
    }

    fn receive_packet(&mut self) -> ComResult<CEPPacket> {
        let mut header_buffer = [0; 1];
        self.read_exact(&mut header_buffer)?;

        let header = CEPPacketHeader::from_repr(header_buffer[0] as usize)
            .ok_or(CommunicationError::PacketInvalidError)?;
        let packet = match header {
            CEPPacketHeader::ACK => CEPPacket::ACK,
            CEPPacketHeader::NACK => CEPPacket::NACK,
            CEPPacketHeader::STOP => CEPPacket::STOP,
            CEPPacketHeader::EOF => CEPPacket::EOF,
            CEPPacketHeader::DATA => {
                let mut length_buffer = [0; 2];
                self.read_exact(&mut length_buffer)?;
                let length = u16::from_le_bytes(length_buffer);

                let mut data_buffer = vec![0; length as usize];
                self.read_exact(&mut data_buffer)?;

                let mut crc_buffer = [0; 4];
                self.read_exact(&mut crc_buffer)?;
                if !CEPPacket::crc_is_valid(&data_buffer, u32::from_le_bytes(crc_buffer)) {
                    return Err(CommunicationError::CRCError);
                }

                self.send_packet(&CEPPacket::ACK)?;
                CEPPacket::DATA(data_buffer)
            }
        };

        //self.set_timeout(&Duration::MAX);
        Ok(packet)
    }

    fn receive_multi_packet(&mut self, stop_fn: impl Fn() -> bool) -> ComResult<Vec<u8>> {
        let mut buffer = Vec::new();

        loop {
            let pack = self.receive_packet();
            if stop_fn() {
                self.send_packet(&CEPPacket::STOP)?;
                return Err(CommunicationError::STOPCondition);
            }

            match pack {
                Ok(CEPPacket::DATA(b)) => {
                    buffer.extend(b);
                }
                Ok(CEPPacket::EOF) => {
                    break;
                }
                Ok(CEPPacket::STOP) => {
                    return Err(CommunicationError::STOPCondition);
                }
                Err(e @ CommunicationError::Io(_)) => {
                    return Err(e);
                }
                Err(CommunicationError::TimedOut) => {
                    log::error!("Receive multipacket timed out");
                    return Err(CommunicationError::TimedOut);
                }
                e => {
                    log::error!("Received invalid data {:?}", e);
                    self.send_packet(&CEPPacket::NACK)?;
                }
            };
        }

        self.send_packet(&CEPPacket::ACK)?;
        Ok(buffer)
    }

    fn await_ack(&mut self, timeout: &Duration) -> ComResult<()> {
        self.set_timeout(timeout);
        match self.receive_packet()? {
            CEPPacket::ACK => Ok(()),
            CEPPacket::NACK => Err(CommunicationError::NotAcknowledged),
            _ => Err(CommunicationError::PacketInvalidError)
        }
    }
}

impl CommunicationHandle for Box<dyn serialport::SerialPort> {
    const INTEGRITY_ACK_TIMEOUT: Duration = Duration::from_millis(100);
    const UNLIMITED_TIMEOUT: Duration = Duration::MAX;

    fn set_timeout(&mut self, timeout: &Duration) {
        serialport::SerialPort::set_timeout(self.as_mut(), *timeout).unwrap()
    }
}

#[derive(Debug)]
pub enum CommunicationError {
    /// Signals that an unknown command packet was received
    PacketInvalidError,
    /// Signals that the CRC checksum of a data packet was wrong
    CRCError,
    /// Signals that the underlying sending or receiving failed. Not recoverable on its own.
    Io(std::io::Error),
    /// Signals that a multi packet receive or send was interrupted by a STOP condition
    STOPCondition,
    /// Signals that a receive timed out
    TimedOut,
    /// NACK was received when ACK was expected
    NotAcknowledged
}

impl std::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for CommunicationError {
    fn from(value: std::io::Error) -> Self {
        if value.kind() == std::io::ErrorKind::TimedOut {
            CommunicationError::TimedOut
        } else {
            CommunicationError::Io(value)
        }
    }
}

impl std::error::Error for CommunicationError {}

#[cfg(test)]
pub mod tests {
    use super::*;
    use test_case::test_case;

    #[derive(Default)]
    pub struct TestComHandle {
        pub written_data: Vec<u8>,
        pub data_to_read: Vec<u8>,
    }
    impl Read for TestComHandle {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            buf.copy_from_slice(&self.data_to_read[0..buf.len()]);
            self.data_to_read.drain(0..buf.len());
            Ok(buf.len())
        }
    }
    impl Write for TestComHandle {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written_data.extend(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl CommunicationHandle for TestComHandle {
        const INTEGRITY_ACK_TIMEOUT: Duration = Duration::from_millis(100);
        const UNLIMITED_TIMEOUT: Duration = Duration::MAX;
        fn set_timeout(&mut self, _timeout: &Duration) {}
    }

    #[test_case(CEPPacket::ACK)]
    #[test_case(CEPPacket::NACK)]
    #[test_case(CEPPacket::STOP)]
    #[test_case(CEPPacket::EOF)]
    #[test_case(CEPPacket::DATA(vec![1, 2, 3]))]

    fn packet_is_sent_correctly(packet: CEPPacket) {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut CEPPacket::ACK.serialize());

        com.send_packet(&packet).unwrap();

        assert_eq!(com.written_data, packet.serialize());
    }

    #[test_case(CEPPacket::ACK)]
    #[test_case(CEPPacket::NACK)]
    #[test_case(CEPPacket::STOP)]
    #[test_case(CEPPacket::EOF)]
    #[test_case(CEPPacket::DATA(vec![1, 2, 3]))]
    fn packet_is_received_correctly(packet: CEPPacket) {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut packet.clone().serialize());

        assert_eq!(com.receive_packet().unwrap(), packet);

        if matches!(packet, CEPPacket::DATA(_)) {
            assert_eq!(com.written_data, CEPPacket::ACK.serialize());
        }
    }

    #[test]
    fn error_on_nack() {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut CEPPacket::NACK.serialize());

        let ret = com.send_packet(&CEPPacket::DATA(vec![1, 2, 3])).unwrap_err();
        assert!(matches!(ret, CommunicationError::NotAcknowledged));
    }
}
