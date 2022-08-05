use std::collections::VecDeque;

use STS1_EDU_Scheduler::{communication::{CommunicationHandle, CommunicationError, ComResult, CSBIPacket}, command::ExecutionContext};

#[derive(Debug)]
pub enum ComEvent {
    COBC(CSBIPacket),
    EDU(CSBIPacket),
    SLEEP(std::time::Duration)
}

pub struct TestCom {
    expected_events: Vec<ComEvent>,
    receive_queue: Vec<u8>,
    index: usize
}

impl CommunicationHandle for TestCom {
    fn send(&mut self, mut bytes: Vec<u8>) -> ComResult<()> {
        if let ComEvent::EDU(p) = &self.expected_events[self.index] {
            assert_eq!(bytes, p.clone().serialize());
            self.index += 1;
            Ok(())
        }
        else {
            panic!("EDU should not send now, expected {}: {:?}", self.index, self.expected_events[self.index]);
        }
    }

    fn receive(&mut self, n: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        if !self.receive_queue.is_empty() {
            let res: Vec<u8> = self.receive_queue.drain(0..(n as usize)).collect();
            if self.receive_queue.is_empty() {
                self.index += 1;
            }
            return Ok(res);
        }

        if let ComEvent::SLEEP(d) = &self.expected_events[self.index] {
            std::thread::sleep(*d);
            self.index += 1;
        }

        if let ComEvent::COBC(p) = &self.expected_events[self.index] {
            self.receive_queue.append(&mut p.clone().serialize());
            self.receive(n, timeout)
        }
        else {
            panic!("EDU should send now, expected {}: {:?}", self.index, self.expected_events[self.index]);
        }
    }
}

impl TestCom {
    pub fn new(packets: Vec<ComEvent>) -> Self {
        TestCom { expected_events: packets, receive_queue: vec![], index: 0 }
    }

    pub fn is_complete(&self) -> bool {
        self.index == self.expected_events.len()
    }
}

pub fn prepare_program(path: &str) {
    let ret = std::fs::create_dir(format!("./archives/{}", path));
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
    let ret = std::fs::copy("./tests/test_data/main.py", format!("./archives/{}/main.py", path));
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
}

pub fn prepare_handles(packets: Vec<ComEvent>, unique: &str) -> (TestCom, ExecutionContext) {
    let com = TestCom::new(packets);
    let exec = ExecutionContext::new(format!("{}_s", unique).into(), format!("{}_r", unique).into()).unwrap();

    return (com, exec);
}

pub fn cleanup(unique: &str) {
    std::fs::remove_dir_all(format!("./archives/{}", unique));
    std::fs::remove_file(format!("{}_s", unique));
    std::fs::remove_file(format!("{}_r", unique));
}