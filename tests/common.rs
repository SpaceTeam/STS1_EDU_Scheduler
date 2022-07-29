use std::collections::VecDeque;

use STS1_EDU_Scheduler::communication::{CommunicationHandle, CommunicationError, ComResult, CSBIPacket};

#[derive(Debug)]
pub enum ComEvent {
    COBC(CSBIPacket),
    EDU(CSBIPacket)
}

pub struct TestCom {
    expected_events: Vec<ComEvent>,
    receive_queue: Vec<u8>,
    index: usize
}

impl CommunicationHandle for TestCom {
    fn send(&mut self, mut bytes: Vec<u8>) -> ComResult<()> {
        if let ComEvent::EDU(p) = &self.expected_events[self.index] {
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
        self.expected_events.is_empty()
    }
}

