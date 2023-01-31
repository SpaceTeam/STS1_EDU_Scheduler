use crate::persist::{FileQueue, Serializable};
use std::{
    sync::{Arc, Mutex},
    thread,
};

/// This type makes the ExecutionContext thread-safe
pub type SyncExecutionContext = Arc<Mutex<ExecutionContext>>;

/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    /// Contains the JoinHandle of the watchdog thread
    pub thread_handle: Option<thread::JoinHandle<()>>,
    /// Through this value, the watchdog thread indicates, wether a student program is currently
    /// running. Changing it from true to false, indicates to the watchdog thread, that the
    /// program should be stopped
    pub running_flag: bool,
    /// This queue contains information about finished student programs, that is to be sent to
    /// the COBC  
    pub status_queue: FileQueue<ProgramStatus>,
    /// This queue contains information about results, that should be sent to the COBC
    pub result_queue: FileQueue<ResultId>,
    /// This integer is the pin number of the EDU_Update pin
    pub update_pin: UpdatePin,
}

impl ExecutionContext {
    pub fn new(
        status_path: std::path::PathBuf,
        result_path: std::path::PathBuf,
        update_pin: u8,
    ) -> Result<Self, std::io::Error> {
        let mut ec = ExecutionContext {
            thread_handle: None,
            running_flag: false,
            status_queue: FileQueue::<ProgramStatus>::new(status_path)?,
            result_queue: FileQueue::<ResultId>::new(result_path)?,
            update_pin: UpdatePin::new(update_pin),
        };

        if ec.has_data_ready()? {
            ec.update_pin.set_high();
        } else {
            ec.update_pin.set_low();
        }

        Ok(ec)
    }

    pub fn is_student_program_running(&self) -> bool {
        self.running_flag
    }

    pub fn has_data_ready(&self) -> Result<bool, std::io::Error> {
        Ok(!self.status_queue.is_empty()? || !self.result_queue.is_empty()?)
    }
}

#[cfg(not(feature = "mock"))]
pub struct UpdatePin {
    pub pin: rppal::gpio::OutputPin,
}

#[cfg(not(feature = "mock"))]
impl UpdatePin {
    pub fn new(pin: u8) -> Self {
        let mut update_pin =
            UpdatePin { pin: rppal::gpio::Gpio::new().unwrap().get(pin).unwrap().into_output() };
        update_pin.pin.set_reset_on_drop(false);
        update_pin
    }

    pub fn set_high(&mut self) {
        self.pin.set_high();
    }

    pub fn set_low(&mut self) {
        self.pin.set_low();
    }
}

/// This impl is only used when doing tests without hardware
#[cfg(feature = "mock")]
pub struct UpdatePin {
    pub pin: bool,
}

#[cfg(feature = "mock")]
impl UpdatePin {
    pub fn new(pin: u8) -> Self {
        let update_pin = UpdatePin { pin: false };
        return update_pin;
    }

    pub fn set_high(&mut self) {
        self.pin = true
    }

    pub fn set_low(&mut self) {
        self.pin = false
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

/// This impl allows ProgramStatus to be used in a FileQueue
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
        let p_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let q_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        ProgramStatus { program_id: p_id, queue_id: q_id, exit_code: bytes[4] }
    }
}

/// This impl allows ResultId to be used in a FileQueue
impl Serializable for ResultId {
    const SIZE: usize = 4;

    fn serialize(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(self.program_id.serialize());
        v.extend(self.queue_id.serialize());
        v
    }

    fn deserialize(bytes: &[u8]) -> Self {
        let p_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let q_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        ResultId { program_id: p_id, queue_id: q_id }
    }
}
