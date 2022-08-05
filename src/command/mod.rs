use crate::communication::{CommunicationHandle, CSBIPacket, CommunicationError};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, atomic};

mod handlers;
use handlers::*;

type CommandResult = Result<(), CommandError>;

const COM_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::new(2, 0);

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn process_command(com: &mut impl CommunicationHandle, exec: &mut Option<ExecutionContext>) -> CommandResult {
    // Preprocess
    let packet = com.receive_packet(&COM_TIMEOUT_DURATION)?;
    let data = if let CSBIPacket::DATA(bytes) = packet {
        bytes
    }
    else {
        return Err(CommandError::InvalidCommError); // Did not start with a data packet
    };

    if data.len() < 1 {
        return Err(CommandError::InvalidCommError);
    }

    match data[0] {
        0x01 => { // STORE ARCHIVE
            if data.len() != 3 {
                return Err(CommandError::InvalidCommError);
            }
            com.send_packet(CSBIPacket::ACK)?;
            let id = u16::from_be_bytes([data[1], data[2]]).to_string();
            let bytes = com.receive_multi_packet(&COM_TIMEOUT_DURATION, || {false})?; // !! TODO !!
            store_archive(id, bytes)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x02 => { // EXECUTE PROGRAM
            if data.len() != 7 {
                return Err(CommandError::InvalidCommError);
            }
            com.send_packet(CSBIPacket::ACK)?;
            let program_id = u16::from_be_bytes([data[1], data[2]]).to_string();
            let queue_id = u16::from_be_bytes([data[3], data[4]]).to_string();
            let timeout = Duration::from_secs(u16::from_be_bytes([data[5], data[6]]).into());
            execute_program(exec, &program_id, &queue_id, &timeout)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x03 => { // STOP PROGRAM
            if data.len() != 1 {
                return Err(CommandError::InvalidCommError);
            }
            stop_program(exec)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x04 => { // GET STATUS
            if data.len() != 1 {
                return Err(CommandError::InvalidCommError);
            }
            com.send_packet(CSBIPacket::ACK)?;
            com.send_packet(get_status()?)?;
            com.receive_packet(&COM_TIMEOUT_DURATION)?; // Throw away ACK
        },
        0x05 => { // RETURN RESULT
            if data.len() != 1 {
                return Err(CommandError::InvalidCommError);
            }
            com.send_packet(CSBIPacket::ACK)?;
            com.send_multi_packet(return_result()?)?;
            if let CSBIPacket::ACK = com.receive_packet(&COM_TIMEOUT_DURATION)? {
                delete_result()?;
            }
        },
        0x06 => { // UPDATE TIME
            if data.len() != 5 {
                return Err(CommandError::InvalidCommError);
            }
            com.send_packet(CSBIPacket::ACK)?;
            let time = i32::from_be_bytes([data[1], data[2], data[3], data[4]]);
            update_time(time)?;
            com.send_packet(CSBIPacket::ACK)?;
        }
        _ => {
            return Err(CommandError::InvalidCommError);
        }
    };
    
    return Ok(());
}


/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    pub thread_handle: thread::JoinHandle<()>,
    pub running_flag: Arc<atomic::AtomicBool>,
}

impl ExecutionContext {
    pub fn is_running(&self) -> bool {
        return self.running_flag.load(atomic::Ordering::Relaxed);
    }
}


#[derive(Debug)]
pub enum CommandError {
    /// Propagates an error from the communication module
    CommunicationError(CommunicationError),
    /// Signals that something has gone wrong while using a system tool (e.g. unzip)
    SystemError(Box<dyn std::error::Error>),
    /// Signals that packets that are not useful right now were received
    InvalidCommError
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<subprocess::PopenError> for CommandError {
    fn from(e: subprocess::PopenError) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<CommunicationError> for CommandError {
    fn from(e: CommunicationError) -> Self {
        CommandError::CommunicationError(e)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CommandError {}