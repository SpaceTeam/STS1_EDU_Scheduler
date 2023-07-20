use super::CSBIPacket;

pub type ComResult<T> = Result<T, CommunicationError>;

pub trait CommunicationHandle {
    /// Sends the bytes to the COBC, packaged accordingly. This function shall block until all data
    /// is sent. By returning a [`CommunicationError::InterfaceError`] it can signal that the underlying driver failed.
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()>;

    /// Blocks until byte_count are received or the timeout is reached. A [`CommunicationError`] can signal that it failed
    /// or timed out.
    fn receive(&mut self, byte_count: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>>;

    /// Sends the supplied packet
    fn send_packet(&mut self, p: CSBIPacket) -> ComResult<()> {
        if matches!(&p, CSBIPacket::DATA(_)) {
            loop {
                self.send(p.clone().serialize())?;
                match self.receive_packet(&std::time::Duration::MAX) {
                    Ok(CSBIPacket::ACK) => break,
                    Ok(CSBIPacket::NACK) => log::warn!("Received NACK after DATA, resending..."),
                    Ok(CSBIPacket::STOP) => return Err(CommunicationError::STOPCondition),
                    Ok(_) => return Err(CommunicationError::PacketInvalidError),
                    Err(e) => return Err(e),
                }
            }
        } else {
            self.send(p.serialize())?;
        }

        Ok(())
    }

    /// Blocks until it receives a CSBIPacket
    fn receive_packet(&mut self, timeout: &std::time::Duration) -> ComResult<CSBIPacket> {
        let pack = match self.receive(1, timeout)?[0] {
            0xd7 => CSBIPacket::ACK,
            0x27 => CSBIPacket::NACK,
            0xb4 => CSBIPacket::STOP,
            0x59 => CSBIPacket::EOF,
            0x8b => {
                let length_field = self.receive(2, timeout)?;
                let length = u16::from_le_bytes([length_field[0], length_field[1]]);
                let bytes = self.receive(length, timeout)?;
                let crc_field = self.receive(4, timeout)?;
                let crc =
                    u32::from_le_bytes([crc_field[0], crc_field[1], crc_field[2], crc_field[3]]);
                if !CSBIPacket::check(&bytes, crc) {
                    return Err(CommunicationError::CRCError);
                } else {
                    CSBIPacket::DATA(bytes)
                }
            }
            _ => {
                return Err(CommunicationError::PacketInvalidError);
            }
        };

        Ok(pack)
    }

    /// Attempts to continously receive multidata packets and returns them in a concatenated byte vector
    /// `stop_fn` is evaluated after every packet and terminates the communication with a STOP packet if true
    /// An error is returned in this case
    fn receive_multi_packet(
        &mut self,
        timeout: &std::time::Duration,
        stop_fn: impl Fn() -> bool,
    ) -> ComResult<Vec<u8>> {
        let mut buffer = Vec::new();

        loop {
            let pack = self.receive_packet(timeout);
            if stop_fn() {
                self.send_packet(CSBIPacket::STOP)?;
                return Err(CommunicationError::STOPCondition);
            }

            match pack {
                Ok(CSBIPacket::DATA(b)) => {
                    buffer.extend(b);
                    self.send_packet(CSBIPacket::ACK)?;
                }
                Ok(CSBIPacket::EOF) => {
                    break;
                }
                Ok(CSBIPacket::STOP) => {
                    return Err(CommunicationError::STOPCondition);
                }
                Err(CommunicationError::InterfaceError) => {
                    return Err(CommunicationError::InterfaceError);
                }
                Err(CommunicationError::TimeoutError) => {
                    log::error!("Receive multipacket timed out");
                    return Err(CommunicationError::TimeoutError);
                }
                e => {
                    log::error!("Received invalid data {:?}", e);
                    self.send_packet(CSBIPacket::NACK)?;
                }
            };
        }

        Ok(buffer)
    }

    fn send_multi_packet(
        &mut self,
        bytes: Vec<u8>,
        timeout: &std::time::Duration,
    ) -> ComResult<()> {
        let num_packets = bytes.len() / 32768 + 1;
        let chunks: Vec<&[u8]> = bytes.chunks(32768).collect();

        let mut i = 0;
        loop {
            if i == num_packets {
                break;
            }

            self.send_packet(CSBIPacket::DATA(chunks[i].to_vec()))?;
            i += 1;
        }

        self.send_packet(CSBIPacket::EOF)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum CommunicationError {
    /// Signals that an unknown command packet was received
    PacketInvalidError,
    /// Signals that the CRC checksum of a data packet was wrong
    CRCError,
    /// Signals that the underlying sending or receiving failed. Not recoverable on its own.
    InterfaceError,
    /// Signals that a multi packet receive or send was interrupted by a STOP condition
    STOPCondition,
    /// Signals that a receive timed out
    TimeoutError,
}

impl std::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CommunicationError {}
