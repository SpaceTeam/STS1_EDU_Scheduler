use std::time::Duration;

use crate::communication::{ComResult, CommunicationHandle};
use rppal::uart::{Parity, Uart};

use super::CommunicationError;

//Constants
const DATA_BITS: u8 = 8;
const STOP_BITS: u8 = 1;
const ALLOWED_SEND_RETRIES: u8 = 3;

pub struct UARTHandle {
    uart_PI: Uart,
}

impl UARTHandle {
    /// # ISSUE!
    /// By Default UART Pins initiated this way will be TX: 14 and RX: 15. This is not coherent with PDD.
    /// Pins need to be changed somehow
    ///
    /// ## Arguments
    /// * `baud` - Bits per second
    ///
    /// ## Returns:
    /// A `UARTHandle`(r) that uses Raspberry Pi's UART Peripheral
    ///
    pub fn new(baud: u32) -> UARTHandle {
        let mut uart_handler: UARTHandle = UARTHandle {
            uart_PI: Uart::new(baud, Parity::None, DATA_BITS, STOP_BITS).unwrap(),
        };

        let _ = uart_handler.uart_PI.set_write_mode(true);

        return uart_handler;
    }
}

impl CommunicationHandle for UARTHandle {
    /// Sends the given bytes via UART
    /// ## Arguments
    /// * `bytes` - a raw byte vector to send
    /// ## Returns
    /// * `Ok`: Sent correctly
    /// * `InterfaceError`: UART Peripheral returns an error or failed to send everything
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {
        let mut sent_bytes: usize = 0;

        for _ in 0..=ALLOWED_SEND_RETRIES {
            sent_bytes += self.uart_PI.write(&bytes[sent_bytes..])?;

            if sent_bytes == bytes.len() {
                return Ok(());
            }
        }
        return Err(CommunicationError::InterfaceError);
    }

    /// # Incomplete
    /// Does not honor the supplied timeout. Planned for after HAF
    ///
    /// Receives a byte packet of some exepected length with a timeout (non inter-byte). Function while retry after failed attemps indefinitely during the given timeout
    /// while the whole amount of expected bytes hasn't been received.
    /// ## Arguments
    /// * `byte_count`: the amount of bytes that is expected
    /// * `timeout`: timeout for the set_read_mode from rppal. Global time of the function. Not inter-byte
    /// ## Returns
    /// A vector of bytes
    fn receive(&mut self, byte_count: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        //Data buffer
        let mut received_data_buffer: Vec<u8> = vec![0; byte_count as usize];

        let mut received_bytes_counter: usize = 0;
        let mut read_byte_count: u8;

        while received_bytes_counter < byte_count as usize {
            read_byte_count = std::cmp::min(byte_count, 255) as u8;
            self.uart_PI
                .set_read_mode(read_byte_count, Duration::ZERO)?;

            received_bytes_counter += self
                .uart_PI
                .read(&mut received_data_buffer[received_bytes_counter..])?;
        }

        Ok(received_data_buffer)
    }
}

impl From<rppal::uart::Error> for CommunicationError {
    fn from(_: rppal::uart::Error) -> Self {
        CommunicationError::InterfaceError
    }
}
