use std::sync::{Arc, Mutex};

use STS1_EDU_Scheduler::{
    command::{ExecutionContext, SyncExecutionContext},
    communication::{CSBIPacket, ComResult, CommunicationHandle},
};

pub enum ComEvent {
    /// EDU shall want to receive the given packet
    COBC(CSBIPacket),
    COBC_INVALID(Vec<u8>),
    /// EDU shall send the given packet
    EDU(CSBIPacket),
    /// Makes the thread sleep for the given duration. Can be used to wait for execution to complete
    SLEEP(std::time::Duration),
    /// Allow the EDU to send any packet
    ANY,
    /// EDU shall send a packet, which is then passed to a given function (e.g. to allow for further checks on data)
    ACTION(Box<dyn Fn(Vec<u8>)>),
}

/// This communciation handle can simulate what is going on between EDU and COBC. Any send or receive call is
/// checked against the supplied expected events vector
pub struct TestCom {
    expected_events: Vec<ComEvent>,
    receive_queue: Vec<u8>,
    index: usize,
}

impl CommunicationHandle for TestCom {
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {
        match &self.expected_events[self.index] {
            ComEvent::EDU(p) => {
                assert_eq!(bytes, p.clone().serialize(), "Wrong packet {}", self.index);
                self.index += 1;
                Ok(())
            }
            ComEvent::SLEEP(d) => {
                std::thread::sleep(*d);
                self.index += 1;
                Ok(())
            }
            ComEvent::ANY => {
                self.index += 1;
                Ok(())
            }
            ComEvent::ACTION(f) => {
                f(bytes);
                self.index += 1;
                Ok(())
            }
            _ => {
                panic!("EDU should not send now, index {}", self.index);
            }
        }
    }

    fn receive(&mut self, n: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        match &self.expected_events[self.index] {
            ComEvent::COBC(p) => {
                if !self.receive_queue.is_empty() {
                    let res: Vec<u8> = self.receive_queue.drain(0..(n as usize)).collect();
                    if self.receive_queue.is_empty() {
                        self.index += 1;
                    }
                    Ok(res)
                } else {
                    self.receive_queue.append(&mut p.clone().serialize());
                    self.receive(n, timeout)
                }
            }
            ComEvent::COBC_INVALID(b) => {
                if !self.receive_queue.is_empty() {
                    let res: Vec<u8> = self.receive_queue.drain(0..(n as usize)).collect();
                    if self.receive_queue.is_empty() {
                        self.index += 1;
                    }
                    Ok(res)
                } else {
                    self.receive_queue.append(&mut b.clone());
                    self.receive(n, timeout)
                }
            }
            ComEvent::SLEEP(d) => {
                std::thread::sleep(*d);
                self.index += 1;
                self.receive(n, timeout)
            }
            _ => {
                panic!("EDU should send now, index {}", self.index);
            }
        }
    }
}

impl TestCom {
    pub fn new(packets: Vec<ComEvent>) -> Self {
        TestCom {
            expected_events: packets,
            receive_queue: vec![],
            index: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.index == self.expected_events.len()
    }
}

/// Copy the mockup student program from `tests/test_data/main.py` into `archives/{path}`. This absolves the need
/// to include an extra store_archive command.
pub fn prepare_program(path: &str) {
    let ret = std::fs::create_dir(format!("./archives/{}", path));
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
    let ret = std::fs::copy(
        "./tests/test_data/main.py",
        format!("./archives/{}/main.py", path),
    );
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
}

/// Construct a communication and execution handle for testing.
/// * `packets` is a vector of expected communication see [ComEvent] for documentation
/// * `unique` A string that is unique among other tests. Can be a simple incrementing number
pub fn prepare_handles(packets: Vec<ComEvent>, unique: &str) -> (TestCom, SyncExecutionContext) {
    let _ = std::fs::create_dir("tests/tmp");
    file_per_thread_logger::allow_uninitialized();
    file_per_thread_logger::initialize("tests/tmp/log-");
    let com = TestCom::new(packets);
    let ec = ExecutionContext::new(
        format!("tests/tmp/{}_s", unique).into(),
        format!("tests/tmp/{}_r", unique).into(),
        12,
    )
    .unwrap();
    let exec = Arc::new(Mutex::new(ec));

    return (com, exec);
}

pub fn cleanup(unique: &str) {
    let _ = std::fs::remove_dir_all(format!("./archives/{}", unique));
    let _ = std::fs::remove_file(format!("tests/tmp/{}_s", unique));
    let _ = std::fs::remove_file(format!("tests/tmp/{}_r", unique));
}
