use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use std::io::Read;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn execute_program_normal() -> TestResult {
    let packets = vec![
        Cobc(Data(execute_program(1, 0, 2))), // Execute Program ID 1, Timestamp 0, Timeout 2s
        Edu(Ack),
        Edu(Ack),
    ];
    common::prepare_program("1");
    let (mut com, mut exec) = common::prepare_handles(packets, "1");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(500));
    let result_file = std::fs::read("./data/1_0")?;
    assert!(result_file.windows(38).any(|w| w == b"Some test results\nWith multiple lines\n"));

    common::cleanup("1");
    Ok(())
}

#[test]
fn execute_program_infinite() {
    let packets = vec![
        Cobc(Data(execute_program(2, 1, 1))), // Execute Program ID 2, Timestamp 1, Timeout 1s
        Edu(Ack),
        Edu(Ack),
        Cobc(Data(get_status())),
        Edu(Ack),
        Edu(Data(vec![1, 2, 0, 1, 0, 0, 0, 255])),
        Cobc(Ack),
    ];
    common::prepare_program("2");
    let (mut com, mut exec) = common::prepare_handles(packets, "2");

    command::handle_command(&mut com, &mut exec);
    std::thread::sleep(std::time::Duration::from_millis(1300));
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("2");
}

#[test]
fn execute_missing_program() {
    let packets = vec![Cobc(Data(execute_program(11, 0, 2))), Edu(Ack), Edu(Nack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "12");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("12");
}
