use std::{
    collections::VecDeque,
    fmt::Debug,
    io::{Read, Write},
    time::Duration,
};

use STS1_EDU_Scheduler::{
    command::{ExecutionContext, SyncExecutionContext},
    communication::{CEPPacket, ComResult, CommunicationHandle},
};

pub enum ComEvent {
    /// EDU shall want to receive the given packet
    Cobc(CEPPacket),
    /// EDU shall send the given packet
    Edu(CEPPacket),
    /// Makes the thread sleep for the given duration. Can be used to wait for execution to complete
    Sleep(std::time::Duration),
    /// Allow the EDU to send any packet
    Any,
    /// EDU shall send a packet, which is then passed to a given function (e.g. to allow for further checks on data)
    Action(Box<dyn Fn(&CEPPacket)>),
}

impl Debug for ComEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cobc(arg0) => f.debug_tuple("COBC").field(arg0).finish(),
            Self::Edu(arg0) => f.debug_tuple("EDU").field(arg0).finish(),
            Self::Sleep(arg0) => f.debug_tuple("SLEEP").field(arg0).finish(),
            Self::Any => write!(f, "ANY"),
            Self::Action(_) => f.debug_tuple("ACTION").finish(),
        }
    }
}

/// This communciation handle can simulate what is going on between EDU and COBC. Any send or receive call is
/// checked against the supplied expected events vector
pub struct TestCom {
    expected_events: VecDeque<ComEvent>,
}

impl CommunicationHandle for TestCom {
    fn send_packet(&mut self, packet: &CEPPacket) -> ComResult<()> {
        println!("Sent {packet:?}");
        match self.expected_events.pop_front().unwrap() {
            ComEvent::Edu(p) => assert_eq!(&p, packet),
            ComEvent::Sleep(d) => std::thread::sleep(d),
            ComEvent::Any => (),
            ComEvent::Action(f) => f(packet),
            event @ ComEvent::Cobc(_) => panic!("Expected {event:?} instead of send_packet"),
        }

        if matches!(packet, CEPPacket::Data(_)) {
            self.await_ack(Self::INTEGRITY_ACK_TIMEOUT)?;
        }

        Ok(())
    }

    fn receive_packet(&mut self) -> ComResult<CEPPacket> {
        match self.expected_events.pop_front().unwrap() {
            ComEvent::Cobc(p) => {
                println!("Received {p:?}");
                if matches!(p, CEPPacket::Data(_)) {
                    self.send_packet(&CEPPacket::Ack)?;
                }
                Ok(p)
            }
            ComEvent::Sleep(d) => {
                std::thread::sleep(d);
                self.receive_packet()
            }
            event => panic!("Expected {event:?} instead of receive_packet"),
        }
    }

    const INTEGRITY_ACK_TIMEOUT: std::time::Duration = Duration::MAX;
    const UNLIMITED_TIMEOUT: std::time::Duration = Duration::MAX;

    fn set_timeout(&mut self, _timeout: std::time::Duration) {}
}

impl TestCom {
    pub fn new(packets: Vec<ComEvent>) -> Self {
        TestCom { expected_events: packets.into() }
    }

    pub fn is_complete(&self) -> bool {
        self.expected_events.is_empty()
    }
}

impl Read for TestCom {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
}
impl Write for TestCom {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Copy the mockup student program from `tests/test_data/main.py` into `archives/{path}`. This absolves the need
/// to include an extra `store_archive` command.
pub fn prepare_program(path: &str) {
    let ret = std::fs::create_dir_all(format!("./archives/{path}"));
    if let Err(e) = ret {
        assert!(e.kind() == std::io::ErrorKind::AlreadyExists, "Setup Error: {e}");
    }
    let _ = std::fs::create_dir_all("./data");

    let ret = std::fs::copy("./tests/test_data/main.py", format!("./archives/{path}/main.py"));
    if let Err(e) = ret {
        assert!(e.kind() == std::io::ErrorKind::AlreadyExists, "Setup Error: {e}");
    }
}

/// Construct a communication and execution handle for testing.
/// * `packets` is a vector of expected communication see [`ComEvent`] for documentation
/// * `unique` A string that is unique among other tests. Can be a simple incrementing number
pub fn prepare_handles(packets: Vec<ComEvent>, unique: &str) -> (TestCom, SyncExecutionContext) {
    let _ = std::fs::create_dir("tests/tmp");
    file_per_thread_logger::allow_uninitialized();
    file_per_thread_logger::initialize("tests/tmp/log-");
    let com = TestCom::new(packets);
    let exec = ExecutionContext::new(format!("tests/tmp/{unique}"), 12).unwrap();

    (com, exec)
}

pub fn cleanup(unique: &str) {
    let _ = std::fs::remove_dir_all(format!("./archives/{unique}"));
    let _ = std::fs::remove_file(format!("tests/tmp/{unique}_s"));
    let _ = std::fs::remove_file(format!("tests/tmp/{unique}_r"));
}

#[allow(dead_code)]
pub fn store_archive(program_id: u16) -> Vec<u8> {
    let mut vec = vec![1u8];
    vec.extend(program_id.to_le_bytes());
    vec
}

pub fn execute_program(program_id: u16, timestamp: u32, timeout: u16) -> Vec<u8> {
    let mut vec = vec![2u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec.extend(timeout.to_le_bytes());
    vec
}

pub fn stop_program() -> Vec<u8> {
    vec![3u8]
}

pub fn get_status() -> Vec<u8> {
    vec![4u8]
}

pub fn return_result(program_id: u16, timestamp: u32) -> Vec<u8> {
    let mut vec = vec![5u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec
}
