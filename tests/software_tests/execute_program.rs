use std::io::Read;

use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn execute_program_normal() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(1, 0, 2))), // Execute Program ID 1, Timestamp 0, Timeout 2s
        EDU(Ack),
        EDU(Ack),
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
        COBC(Data(execute_program(2, 1, 1))), // Execute Program ID 2, Timestamp 1, Timeout 1s
        EDU(Ack),
        EDU(Ack),
        COBC(Data(get_status())),
        EDU(Ack),
        EDU(Data(vec![1, 2, 0, 1, 0, 0, 0, 255])),
        COBC(Ack),
    ];
    common::prepare_program("2");
    let (mut com, mut exec) = common::prepare_handles(packets, "2");

    command::handle_command(&mut com, &mut exec);
    std::thread::sleep(std::time::Duration::from_millis(1300));
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("2");
    Ok(())
}

#[test]
fn execute_missing_program() -> TestResult {
    let packets = vec![COBC(Data(execute_program(11, 0, 2))), EDU(Ack), EDU(Nack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "12");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("12");
    Ok(())
}
