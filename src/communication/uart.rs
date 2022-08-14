use crate::communication::{CommunicationHandle, ComResult};

pub struct UARTHandle {}

impl UARTHandle {
    pub fn new(baud: i32) -> UARTHandle {
        todo!();
    }
}

impl CommunicationHandle for UARTHandle {
    fn send(&mut self, bytes: Vec<u8>) -> ComResult<()> {
        todo!();
    }

    fn receive(&mut self, n: u16, timeout: &std::time::Duration) -> ComResult<Vec<u8>> {
        todo!()
    }
}
