use core::time;
use std::time::Duration;

use crate::{
    command::return_result,
    communication::{ComResult, CommunicationHandle},
};
use log::warn;
use rppal::uart::{Parity, Uart};

use super::{communication, CommunicationError};

//Constants
const DATA_BITS: u8 = 8;
const STOP_BITS: u8 = 1;
const ALLOWED_SEND_RETRIES: u8 = 3;
const MAX_READ_TIMEOUT_MILLIS:u64 = 25000;

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
    /// Sends serialized CSBIPackets
    /// ## Arguments
    /// * `Bytes` - Serialized CSBIPacket
    /// ## Returns
    /// * `Ok`: Sent correctly
    /// * `InterfaceError`: UART Peripheral returns an error or a byte wasn't sent
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {
        let mut sent_bytes: usize = 0;

        for _ in 0..(ALLOWED_SEND_RETRIES + 1) {
            sent_bytes += self.uart_PI.write(&[bytes[sent_bytes]])?;

            if sent_bytes == bytes.len() {
                return Ok(());
            }
        }
        return Err(CommunicationError::InterfaceError);
    }

    ///Receives a given amount of bytes and stores in a buffer
    /// # POTENTIAL ISSUE?
    /// Probably needs to be wrapped in a own timeout since rppal's read-timeout will block indefinitely
    /// until at least one byte as been received.
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
        let mut received_data_buffer: Vec<u8> = Vec::new();
        let mut received_bytes_counter: usize = 0;
        received_data_buffer.reserve(byte_count as usize);

        let mut timer = timeout.clone();
        //Time stamp of maximal length of 25 seconds. Used as inter-byte timeout for read_mode
        let mut time_stamp: Duration;

        while !timer.is_zero() {
            time_stamp = std::time::Duration::from_millis(std::cmp::min(
                timeout.div_f32(byte_count as f32).as_millis() as u64,
                MAX_READ_TIMEOUT_MILLIS
            ));
            //Set the blocking conditions to expect progressively fewer bytes and decrease timeout
            self.uart_PI.set_read_mode(
                (byte_count as u8) - (received_bytes_counter as u8), 
                time_stamp
            )?;

            match self.uart_PI.read(&mut [received_data_buffer[received_bytes_counter]]) {
                Ok(new_bytes_count) => {

                    received_bytes_counter += new_bytes_count;
                    
                    if received_bytes_counter as u16 == byte_count {
                        return Ok(received_data_buffer);
                    }
                }
                _ => {}
            }
            //Asume fully used time_stamp and subtract used time_stamp from timer
            timer = timer.saturating_sub(time_stamp);
        }
        return Err(CommunicationError::TimeoutError);
    }

}

impl From<rppal::uart::Error> for CommunicationError {
    fn from(_: rppal::uart::Error) -> Self {
        CommunicationError::InterfaceError
    }
}