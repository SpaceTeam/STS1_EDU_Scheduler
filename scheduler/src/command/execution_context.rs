use filevec::FileVec;
use std::{
    fmt::Display, str::FromStr, sync::{Arc, Mutex}, thread
};

const EVENT_SEND_TRIES: u32 = 5;

/// This type makes the `ExecutionContext` thread-safe
pub type SyncExecutionContext = Arc<Mutex<ExecutionContext>>;

/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    /// Contains the `JoinHandle` of the watchdog thread
    pub thread_handle: Option<thread::JoinHandle<()>>,
    /// Through this value, the watchdog thread indicates, wether a student program is currently
    /// running. Changing it from true to false, indicates to the watchdog thread, that the
    /// program should be stopped
    pub running_flag: bool,
    /// This integer is the pin number of the `EDU_Update` pin
    pub update_pin: UpdatePin,
    /// Vector containing events that should be sent to the COBC
    pub event_vec: FileVec<RetryEvent<Event>>,
}

impl ExecutionContext {
    pub fn new(
        event_file_path: String,
        update_pin: u8,
    ) -> Result<Arc<Mutex<Self>>, std::io::Error> {
        let mut ec = ExecutionContext {
            thread_handle: None,
            running_flag: false,
            update_pin: UpdatePin::new(update_pin),
            event_vec: FileVec::open(event_file_path)?,
        };

        ec.configure_update_pin();

        Ok(Arc::new(Mutex::new(ec)))
    }

    /// Checks and sets/resets the update pin accordingly
    pub fn configure_update_pin(&mut self) {
        if self.has_data_ready() {
            self.update_pin.set_high();
        } else {
            self.update_pin.set_low();
        }
    }

    #[must_use]
    pub fn is_student_program_running(&self) -> bool {
        self.thread_handle.is_some()
    }

    #[must_use]
    pub fn has_data_ready(&self) -> bool {
        !self.event_vec.as_ref().is_empty()
    }
}

#[cfg(not(feature = "mock"))]
pub struct UpdatePin {
    pub pin: rppal::gpio::OutputPin,
}

#[cfg(not(feature = "mock"))]
impl UpdatePin {
    #[must_use]
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
pub struct UpdatePin;

#[cfg(feature = "mock")]
impl UpdatePin {
    pub fn new(_pin: u8) -> Self {
        Self
    }

    pub fn set_high(&mut self) {
        let _ = std::fs::write("updatepin", b"");
    }

    pub fn set_low(&mut self) {
        let _ = std::fs::remove_file("updatepin");
    }
}

/// Struct used for storing information about a finished student program
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
pub struct ProgramStatus {
    pub program_id: u16,
    pub timestamp: u32,
    pub exit_code: u8,
}

/// Struct used for storing information of a result, waiting to be sent
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
pub struct ResultId {
    pub program_id: u16,
    pub timestamp: u32,
}

impl Display for ResultId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.program_id, self.timestamp)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    Status(ProgramStatus),
    Result(ResultId),
    EnableDosimeter,
    DisableDosimeter,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RetryEvent<T> {
    pub retries: u32,
    pub event: T,
}

impl<T> RetryEvent<T> {
    pub fn new(event: T) -> Self {
        Self { retries: EVENT_SEND_TRIES, event }
    }
}

impl From<Event> for Vec<u8> {
    fn from(value: Event) -> Self {
        let mut v = Vec::new();
        match value {
            Event::Status(s) => {
                v.push(1);
                v.extend(s.program_id.to_le_bytes());
                v.extend(s.timestamp.to_le_bytes());
                v.push(s.exit_code);
            }
            Event::Result(r) => {
                v.push(2);
                v.extend(r.program_id.to_le_bytes());
                v.extend(r.timestamp.to_le_bytes());
            }
            Event::EnableDosimeter => {
                v.push(3);
            }
            Event::DisableDosimeter => {
                v.push(4);
            }
        }
        v
    }
}

impl FromStr for Event {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dosimeter/on" => Ok(Event::EnableDosimeter),
            "dosimeter/off" => Ok(Event::DisableDosimeter),
            _ => Err(()),
        }
    }
}
