use STS1_EDU_Scheduler::{communication::{CommunicationHandle, ComResult, CSBIPacket}, command::ExecutionContext};

pub enum ComEvent {
    COBC(CSBIPacket),
    EDU(CSBIPacket),
    SLEEP(std::time::Duration),
    ANY,
    ACTION(Box<dyn Fn(Vec<u8>)>)
}

pub struct TestCom {
    expected_events: Vec<ComEvent>,
    receive_queue: Vec<u8>,
    index: usize
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
            },
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
                }
                else {
                    self.receive_queue.append(&mut p.clone().serialize());
                    self.receive(n, timeout)
                }
            },
            ComEvent::SLEEP(d) => {
                std::thread::sleep(*d);
                self.index += 1;
                self.receive(n, timeout)
            },
            _ => {
                panic!("EDU should send now, index {}", self.index);
            }
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
    let _ = std::fs::create_dir("tests/tmp");
    let com = TestCom::new(packets);
    let exec = ExecutionContext::new(format!("tests/tmp/{}_s", unique).into(), format!("tests/tmp/{}_r", unique).into()).unwrap();

    return (com, exec);
}

pub fn cleanup(unique: &str) {
    let _ = std::fs::remove_dir_all(format!("./archives/{}", unique));
    let _ = std::fs::remove_file(format!("tests/tmp/{}_s", unique));
    let _ = std::fs::remove_file(format!("tests/tmp/{}_r", unique));
}