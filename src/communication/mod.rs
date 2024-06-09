mod cep;
pub use cep::CEPPacket;

use std::{
    io::{Read, Write},
    time::Duration,
};

use self::cep::CEPParseError;

pub type ComResult<T> = Result<T, CommunicationError>;

pub trait CommunicationHandle: Read + Write {
    const INTEGRITY_ACK_TIMEOUT: Duration;
    const UNLIMITED_TIMEOUT: Duration;

    const DATA_PACKET_RETRIES: usize = 4;

    fn set_timeout(&mut self, timeout: Duration);

    fn send_packet(&mut self, packet: &CEPPacket) -> ComResult<()> {
        let bytes = Vec::from(packet);
        self.write_all(&bytes)?;

        if !(matches!(packet, CEPPacket::Data(_))) {
            return Ok(());
        }

        for i in 1..=Self::DATA_PACKET_RETRIES {
            match self.await_ack(Self::INTEGRITY_ACK_TIMEOUT) {
                Ok(()) => return Ok(()),
                Err(CommunicationError::NotAcknowledged) => {
                    log::warn!("Received NACK, retrying");
                    if i < Self::DATA_PACKET_RETRIES {
                        self.write_all(&bytes)?;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        log::error!("No ACK after {} retries, giving up", Self::DATA_PACKET_RETRIES);
        Err(CommunicationError::PacketInvalidError)
    }

    fn send_multi_packet(&mut self, bytes: &[u8]) -> ComResult<()> {
        let chunks = bytes.chunks(CEPPacket::MAXIMUM_DATA_LENGTH);
        for chunk in chunks {
            self.send_packet(&CEPPacket::Data(chunk.into()))?;
        }

        self.send_packet(&CEPPacket::Eof)?;
        self.await_ack(Self::INTEGRITY_ACK_TIMEOUT)?;

        Ok(())
    }

    fn receive_packet(&mut self) -> ComResult<CEPPacket> {
        for _ in 0..Self::DATA_PACKET_RETRIES {
            match CEPPacket::try_from_read(self) {
                Ok(p @ CEPPacket::Data(_)) => {
                    self.send_packet(&CEPPacket::Ack)?;
                    return Ok(p);
                }
                Ok(p) => return Ok(p),
                Err(CEPParseError::InvalidCRC) => {
                    log::warn!("Received data packet with invalid CRC; Retrying");
                    self.send_packet(&CEPPacket::Nack)?;
                }
                Err(e) => {
                    log::error!("Failed to read packet: {e:?}");
                    return Err(e.into());
                }
            }
        }

        log::error!(
            "Could not receive data packet after {} retries, giving up",
            Self::DATA_PACKET_RETRIES
        );
        Err(CommunicationError::PacketInvalidError)
    }

    fn receive_multi_packet(&mut self) -> ComResult<Vec<u8>> {
        let mut buffer = Vec::new();

        loop {
            let pack = self.receive_packet();

            match pack {
                Ok(CEPPacket::Data(b)) => {
                    buffer.extend(b);
                }
                Ok(CEPPacket::Eof) => {
                    break;
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
                    self.send_packet(&CEPPacket::Nack)?;
                }
            };
        }

        self.send_packet(&CEPPacket::Ack)?;
        Ok(buffer)
    }

    /// Try to receive an ACK packet with a given `timeout`. Resets the timeout to Duration::MAX afterwards
    fn await_ack(&mut self, timeout: Duration) -> ComResult<()> {
        self.set_timeout(timeout);
        let result = self.receive_packet();
        self.set_timeout(Self::UNLIMITED_TIMEOUT);
        let ret = match result? {
            CEPPacket::Ack => Ok(()),
            CEPPacket::Nack => Err(CommunicationError::NotAcknowledged),
            _ => Err(CommunicationError::PacketInvalidError),
        };
        ret
    }
}

impl CommunicationHandle for Box<dyn serialport::SerialPort> {
    const INTEGRITY_ACK_TIMEOUT: Duration = Duration::from_millis(1000);
    /// Equivalent to 106 days, maximum allowed value due to library limitations (of all serialport libraries I found)
    const UNLIMITED_TIMEOUT: Duration = Duration::from_millis(9223372035);

    fn set_timeout(&mut self, timeout: Duration) {
        serialport::SerialPort::set_timeout(self.as_mut(), timeout).unwrap()
    }
}

#[derive(Debug)]
pub enum CommunicationError {
    /// Signals that an unknown command packet was received
    PacketInvalidError,
    /// Relays an error from trying to parse a CEP packet
    CepParsing(CEPParseError),
    /// Signals that the underlying sending or receiving failed. Not recoverable on its own.
    Io(std::io::Error),
    /// Signals that a receive timed out
    TimedOut,
    /// Nack was received when Ack was expected
    NotAcknowledged,
}

impl std::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for CommunicationError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::TimedOut => CommunicationError::TimedOut,
            std::io::ErrorKind::InvalidData => CommunicationError::PacketInvalidError,
            _ => CommunicationError::Io(value),
        }
    }
}

impl From<CEPParseError> for CommunicationError {
    fn from(value: CEPParseError) -> Self {
        match value {
            CEPParseError::Io(e) => e.into(),
            e => Self::CepParsing(e),
        }
    }
}

impl std::error::Error for CommunicationError {}

#[cfg(test)]
mod tests {
    use self::cep::CEPPacketHeader;

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
        fn set_timeout(&mut self, _timeout: Duration) {}
    }

    #[test_case(CEPPacket::Ack)]
    #[test_case(CEPPacket::Nack)]
    #[test_case(CEPPacket::Eof)]
    #[test_case(CEPPacket::Data(vec![1, 2, 3]))]
    fn packet_is_sent_correctly(packet: CEPPacket) {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut CEPPacket::Ack.serialize());

        com.send_packet(&packet).unwrap();

        assert_eq!(com.written_data, packet.serialize());
    }

    #[test_case(CEPPacket::Ack)]
    #[test_case(CEPPacket::Nack)]
    #[test_case(CEPPacket::Eof)]
    #[test_case(CEPPacket::Data(vec![1, 2, 3]))]
    fn packet_is_received_correctly(packet: CEPPacket) {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut packet.clone().serialize());

        assert_eq!(com.receive_packet().unwrap(), packet);

        if matches!(packet, CEPPacket::Data(_)) {
            assert_eq!(com.written_data, CEPPacket::Ack.serialize());
        }
    }

    #[test]
    fn retry_on_nack() {
        let mut com = TestComHandle::default();
        com.data_to_read.append(&mut CEPPacket::Nack.serialize());
        com.data_to_read.append(&mut CEPPacket::Nack.serialize());
        com.data_to_read.append(&mut CEPPacket::Ack.serialize());

        com.send_packet(&CEPPacket::Data(vec![1, 2, 3])).unwrap();

        assert_eq!(com.written_data, CEPPacket::Data(vec![1, 2, 3]).serialize().repeat(3));
        assert!(com.data_to_read.is_empty());
    }

    #[test]
    fn fail_after_retries_send_packet() {
        let mut com = TestComHandle::default();
        for _ in 0..TestComHandle::DATA_PACKET_RETRIES {
            com.data_to_read.append(&mut CEPPacket::Nack.serialize());
        }

        assert!(matches!(
            com.send_packet(&CEPPacket::Data(vec![1, 2, 3])),
            Err(CommunicationError::PacketInvalidError)
        ));
        dbg!(&com.data_to_read);
        assert!(com.data_to_read.is_empty());
        assert_eq!(
            com.written_data,
            CEPPacket::Data(vec![1, 2, 3]).serialize().repeat(TestComHandle::DATA_PACKET_RETRIES)
        );
    }

    #[test]
    fn fail_after_retries_receive_packet() {
        let mut com = TestComHandle::default();
        com.data_to_read.extend(
            [CEPPacketHeader::Data as u8, 1, 0, 2, 1, 1, 1, 1]
                .repeat(TestComHandle::DATA_PACKET_RETRIES),
        );

        let err = com.receive_packet().expect_err("Invalid data packet should fail");
        assert!(matches!(err, CommunicationError::PacketInvalidError));
        assert!(com.data_to_read.is_empty(), "Not read: {:?}", com.data_to_read);
        assert_eq!(
            com.written_data,
            CEPPacket::Nack.serialize().repeat(TestComHandle::DATA_PACKET_RETRIES)
        );
    }

    #[test]
    fn receive_packet_retries_correctly() {
        let mut com = TestComHandle::default();
        com.data_to_read.extend(
            [CEPPacketHeader::Data as u8, 1, 0, 2, 1, 1, 1, 1]
                .repeat(TestComHandle::DATA_PACKET_RETRIES - 1),
        );
        com.data_to_read.append(&mut CEPPacket::Data(vec![2]).serialize());

        assert_eq!(com.receive_packet().unwrap(), CEPPacket::Data(vec![2]));
        assert!(com.data_to_read.is_empty());
        let mut expected =
            CEPPacket::Nack.serialize().repeat(TestComHandle::DATA_PACKET_RETRIES - 1);
        expected.append(&mut CEPPacket::Ack.serialize());
        assert_eq!(com.written_data, expected);
    }

    #[test]
    fn multi_packet_is_sent_correctly() {
        let mut com = TestComHandle::default();

        let data = vec![123u8; 2 * CEPPacket::MAXIMUM_DATA_LENGTH + 50];
        let chunks = data.chunks(CEPPacket::MAXIMUM_DATA_LENGTH);
        com.data_to_read = CEPPacket::Ack.serialize().repeat(chunks.len() + 1);

        com.send_multi_packet(&data).unwrap();

        assert!(com.data_to_read.is_empty());
        for c in chunks {
            assert_eq!(
                com.written_data.drain(0..c.len() + 7).as_slice(),
                CEPPacket::Data(c.to_vec()).serialize()
            );
        }
        assert_eq!(com.written_data, CEPPacket::Eof.serialize());
    }

    #[test]
    fn multi_packet_is_received_correctly() {
        let mut com = TestComHandle::default();

        let data = vec![123u8; 2 * CEPPacket::MAXIMUM_DATA_LENGTH + 50];
        let chunks = data.chunks(CEPPacket::MAXIMUM_DATA_LENGTH);
        for c in chunks.clone() {
            com.data_to_read.append(&mut CEPPacket::Data(c.to_vec()).serialize());
        }
        com.data_to_read.append(&mut CEPPacket::Eof.serialize());

        assert_eq!(com.receive_multi_packet().unwrap(), data);
        assert!(com.data_to_read.is_empty());
        assert_eq!(com.written_data, CEPPacket::Ack.serialize().repeat(chunks.len() + 1))
    }
}
