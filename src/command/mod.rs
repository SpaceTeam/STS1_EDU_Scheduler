use crate::communication::{CommunicationHandle, CSBIPacket, CommunicationError};
use crate::persist::{Serializable, FileQueue};
use std::path::PathBuf;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, atomic, Mutex};

mod handlers;
pub use handlers::*;

type CommandResult = Result<(), CommandError>;

const COM_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::new(2, 0);

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn handle_command(com: &mut impl CommunicationHandle, exec: &mut ExecutionContext) -> CommandResult {
    let ret = process_command(com, exec);

    if let Err(ce) = &ret {
        match ce {
            CommandError::SystemError(_) | CommandError::InvalidCommError => {
                com.send_packet(CSBIPacket::NACK)?;
            }
            _ => {}
        }
    }

    ret
}

pub fn process_command(com: &mut impl CommunicationHandle, exec: &mut ExecutionContext) -> CommandResult {
    // Preprocess
    let packet = com.receive_packet(&Duration::MAX)?;
    let data = if let CSBIPacket::DATA(data) = packet {
        data
    }
    else {
        return Err(CommandError::CommunicationError(CommunicationError::PacketInvalidError)); // Ignore non data packets
    };

    if data.len() < 1 {
        return Err(CommandError::InvalidCommError);
    }

    match data[0] {
        0x01 => { // STORE ARCHIVE
            check_length(&data, 3)?;
            com.send_packet(CSBIPacket::ACK)?;
            let id = u16::from_be_bytes([data[1], data[2]]).to_string();
            let bytes = com.receive_multi_packet(&COM_TIMEOUT_DURATION, || {false})?; // !! TODO !!
            store_archive(id, bytes)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x02 => { // EXECUTE PROGRAM
            check_length(&data, 7)?;
            com.send_packet(CSBIPacket::ACK)?;
            let program_id = u16::from_be_bytes([data[1], data[2]]);
            let queue_id = u16::from_be_bytes([data[3], data[4]]);
            let timeout = Duration::from_secs(u16::from_be_bytes([data[5], data[6]]).into());
            execute_program(exec, program_id, queue_id, timeout)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x03 => { // STOP PROGRAM
            check_length(&data, 1)?;
            com.send_packet(CSBIPacket::ACK)?;
            stop_program(exec)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        0x04 => { // GET STATUS
            check_length(&data, 1)?;
            com.send_packet(CSBIPacket::ACK)?;
            com.send_packet(get_status(exec)?)?;
            com.receive_packet(&COM_TIMEOUT_DURATION)?; // Throw away ACK
        },
        0x05 => { // RETURN RESULT
            check_length(&data, 1)?;
            com.send_packet(CSBIPacket::ACK)?;
            com.send_multi_packet(return_result(exec)?, &COM_TIMEOUT_DURATION)?;
            if let CSBIPacket::ACK = com.receive_packet(&COM_TIMEOUT_DURATION)? {
                delete_result(exec)?;
            }
        },
        0x06 => { // UPDATE TIME
            check_length(&data, 5)?;
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

fn check_length(vec: &Vec<u8>, n: usize) -> Result<(), CommandError> {
    if vec.len() != n {
        Err(CommandError::InvalidCommError)
    }
    else {
        Ok(())
    }
}


pub type SyncExecutionContext = Arc<Mutex<ExecutionContext>>;

/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    pub thread_handle: Option<thread::JoinHandle<()>>,
    pub running_flag: Option<Arc<atomic::AtomicBool>>,
    pub status_q: FileQueue<ProgramStatus>,
    pub result_q: FileQueue<ResultId>
}

impl ExecutionContext {
    pub fn new(status_path: PathBuf, result_path: PathBuf) -> Result<Self, std::io::Error> {
        Ok(ExecutionContext {
            thread_handle: None,
            running_flag: None,
            status_q: FileQueue::<ProgramStatus>::new(status_path)?,
            result_q: FileQueue::<ResultId>::new(result_path)?
        })
    }

    pub fn is_running(&self) -> bool {
        if let Some(f) = &self.running_flag {
            f.load(atomic::Ordering::Relaxed)
        }
        else {
            false
        }
    }
}

/// Struct used for storing information about a finished student program
pub struct ProgramStatus {
    pub program_id: u16,
    pub queue_id: u16,
    pub exit_code: u8,
}

/// Struct used for storing information of a result, waiting to be sent
#[derive(Clone, Copy)]
pub struct ResultId {
    pub program_id: u16,
    pub queue_id: u16
}

impl Serializable for ProgramStatus {
    const SIZE: usize = 5;

    fn serialize(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(self.program_id.serialize());
        v.extend(self.queue_id.serialize());
        v.push(self.exit_code);
        v
    }

    fn deserialize(bytes: &[u8]) -> Self {
        let p_id = u16::from_be_bytes([bytes[0], bytes[1]]);
        let q_id = u16::from_be_bytes([bytes[2], bytes[3]]);
        ProgramStatus { program_id: p_id, queue_id: q_id, exit_code: bytes[4] }
    }
}

impl Serializable for ResultId {
    const SIZE: usize = 4;

    fn serialize(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(self.program_id.serialize());
        v.extend(self.queue_id.serialize());
        v
    }

    fn deserialize(bytes: &[u8]) -> Self {
        let p_id = u16::from_be_bytes([bytes[0], bytes[1]]);
        let q_id = u16::from_be_bytes([bytes[2], bytes[3]]);
        ResultId { program_id: p_id, queue_id: q_id }
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