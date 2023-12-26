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
    const UNLIMITED_TIMEOUT: Duration = Duration::MAX;

    const DATA_PACKET_RETRIES: usize = 4;

    fn set_timeout(&mut self, timeout: &Duration);

    fn send_packet(&mut self, packet: &CEPPacket) -> ComResult<()> {
        let bytes = Vec::from(packet);
        self.write_all(&bytes)?;

        if matches!(packet, CEPPacket::Data(_)) {
            for _ in 0..Self::DATA_PACKET_RETRIES {
                let response = self.receive_packet()?;
                match response {
                    CEPPacket::Ack => return Ok(()),
                    CEPPacket::Nack => log::warn!("Received NACK after data packet; Retrying"),
                    p => {
                        log::error!("Received {p:?} after data packet");
                        return Err(CommunicationError::PacketInvalidError);
                    }
                }

                self.write_all(&bytes)?;
            }
        } else {
            return Ok(());
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
        self.await_ack(&Self::INTEGRITY_ACK_TIMEOUT)?;

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
                    log::warn!("Received data packet with invalid CRC; Retrying")
                }
                Err(e) => {
                    log::error!("Failed to read packet: {e:?}");
                    return Err(e.into());
                }
            }
        }

        todo!()
    }

    fn receive_multi_packet(&mut self, stop_fn: impl Fn() -> bool) -> ComResult<Vec<u8>> {
        let mut buffer = Vec::new();

        loop {
            let pack = self.receive_packet();
            if stop_fn() {
                self.send_packet(&CEPPacket::Stop)?;
                return Err(CommunicationError::StopCondition);
            }

            match pack {
                Ok(CEPPacket::Data(b)) => {
                    buffer.extend(b);
                }
                Ok(CEPPacket::Eof) => {
                    break;
                }
                Ok(CEPPacket::Stop) => {
                    return Err(CommunicationError::StopCondition);
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

    fn await_ack(&mut self, timeout: &Duration) -> ComResult<()> {
        self.set_timeout(timeout);
        match self.receive_packet()? {
            CEPPacket::Ack => Ok(()),
            CEPPacket::Nack => Err(CommunicationError::NotAcknowledged),
            _ => Err(CommunicationError::PacketInvalidError),
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
    /// Relays an error from trying to parse a CEP packet
    CepParsing(CEPParseError),
    /// Signals that the underlying sending or receiving failed. Not recoverable on its own.
    Io(std::io::Error),
    /// Signals that a multi packet receive or send was interrupted by a Stop condition
    StopCondition,
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
            CEPParseError::Io(e) => Self::Io(e),
            e => Self::CepParsing(e),
        }
    }
}

impl std::error::Error for CommunicationError {}

#[cfg(test)]
mod tests {
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

    #[test_case(CEPPacket::Ack)]
    #[test_case(CEPPacket::Nack)]
    #[test_case(CEPPacket::Stop)]
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
    #[test_case(CEPPacket::Stop)]
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

        let mut expected = CEPPacket::Data(vec![1, 2, 3]).serialize();
        expected.extend(CEPPacket::Data(vec![1, 2, 3]).serialize());
        expected.extend(CEPPacket::Data(vec![1, 2, 3]).serialize());
        assert_eq!(com.written_data, expected);
    }

    #[test]
    fn fail_after_retries() {
        let mut com = TestComHandle::default();
        for _ in 0..TestComHandle::DATA_PACKET_RETRIES {
            com.data_to_read.append(&mut CEPPacket::Nack.serialize());
        }

        assert!(
            matches!(
                com.send_packet(&CEPPacket::Data(vec![1, 2, 3])),
                Err(CommunicationError::PacketInvalidError)
            )
        );
    }
}
