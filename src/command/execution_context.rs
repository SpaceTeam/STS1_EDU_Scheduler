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
        todo!();
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProgramStatus {
    pub program_id: u16,
    pub queue_id: u16,
    pub exit_code: u8,
}

/// Struct used for storing information of a result, waiting to be sent
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ResultId {
    pub program_id: u16,
    pub queue_id: u16,
}
