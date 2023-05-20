use std::io::Read;

use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CSBIPacket::*;
mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn execute_program_normal() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00])), // Execute Program ID 1, Queue ID 0, Timeout 2s
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("1");
    let (mut com, mut exec) = common::prepare_handles(packets, "1");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut res = String::new();
    std::fs::File::open("./archives/1/results/0")?.read_to_string(&mut res)?;

    assert_eq!(res.replace('\r', ""), *"Some test results\nWith multiple lines\n".to_string());

    common::cleanup("1");
    Ok(())
}

#[test]
fn execute_program_infinite() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x02, 0x00, 0x01, 0x00, 0x01, 0x00])), // Execute Program ID 2, Queue ID 1, Timeout 1s
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("2");
    let (mut com, mut exec) = common::prepare_handles(packets, "2");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(1300));
    todo!("Ensure there is a status entry");

    common::cleanup("2");
    Ok(())
}

#[test]
fn execute_missing_program() -> TestResult {
    let packets = vec![COBC(DATA(vec![2, 11, 0, 0, 0, 1, 0])), EDU(ACK), EDU(NACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "12");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("12");
    Ok(())
}
