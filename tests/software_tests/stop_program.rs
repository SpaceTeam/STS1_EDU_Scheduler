use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CSBIPacket::*;
mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn stop_program() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x03, 0x00, 0x01, 0x00, 0x0a, 0x00])), // Execute Program 3, Queue 1, Timeout 10s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_secs(1)),
        COBC(DATA(vec![0x03])),
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("3");
    let (mut com, mut exec) = common::prepare_handles(packets, "3");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());
    assert!(!exec.lock().unwrap().status_queue.is_empty()?);

    common::cleanup("3");
    Ok(())
}

#[test]
fn stop_no_running_program() -> TestResult {
    let packets = vec![COBC(DATA(vec![3])), EDU(ACK), EDU(ACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "11");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());
    Ok(())
}
