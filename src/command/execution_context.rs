use std::{sync::{Arc, Mutex}, thread};
use crate::persist::{Serializable, FileQueue};

/// This type makes the ExecutionContext thread-safe
pub type SyncExecutionContext = Arc<Mutex<ExecutionContext>>;

/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    /// Contains the JoinHandle of the watchdog thread
    pub thread_handle: Option<thread::JoinHandle<()>>, 
    /// Through this value, the watchdog thread indicates, wether a student program is currently
    /// running. Changing it from true to false, indicates to the watchdog thread, that the 
    /// program should be stopped
    pub running_flag: Option<bool>,
    /// This queue contains information about finished student programs, that is to be sent to 
    /// the COBC  
    pub status_q: FileQueue<ProgramStatus>,
    /// This queue contains information about results, that should be sent to the COBC
    pub result_q: FileQueue<ResultId>,
    /// This integer is the pin number of the EDU_Update pin
    pub update_pin: u8,
}

impl ExecutionContext {
    pub fn new(
        status_path: std::path::PathBuf,
        result_path: std::path::PathBuf,
        update_pin: u8,
    ) -> Result<Self, std::io::Error> {
        Ok(ExecutionContext {
            thread_handle: None,
            running_flag: None,
            status_q: FileQueue::<ProgramStatus>::new(status_path)?,
            result_q: FileQueue::<ResultId>::new(result_path)?,
            update_pin: update_pin,
        })
    }

    pub fn is_running(&self) -> bool {
        self.running_flag.unwrap_or(false)
    }

    pub fn has_data_ready(&self) -> Result<bool, std::io::Error> {
        Ok(!self.status_q.is_empty()? || !self.result_q.is_empty()?)
    }
}

pub trait UpdatePin {
    fn set_update_high(&self);
    fn set_update_low(&self);
}

#[cfg(not(feature = "mock"))] // Only compile if for target
impl UpdatePin for ExecutionContext {
    fn set_update_high(&self) {
        let mut pin = rppal::gpio::Gpio::new()
            .unwrap()
            .get(self.update_pin)
            .unwrap()
            .into_output();
        pin.set_high();
    }

    fn set_update_low(&self) {
        let mut pin = rppal::gpio::Gpio::new()
            .unwrap()
            .get(self.update_pin)
            .unwrap()
            .into_output();
        pin.set_low();
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
    pub queue_id: u16,
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
        ProgramStatus {
            program_id: p_id,
            queue_id: q_id,
            exit_code: bytes[4],
        }
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
        ResultId {
            program_id: p_id,
            queue_id: q_id,
        }
    }
}
