use std::io::Write;

use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;
mod common;
use common::ComEvent::*;
use common::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn returns_result_correctly() -> TestResult {
    let packets = vec![
        COBC(DATA(execute_program(7, 3, 1))), // Execute Program 7, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(get_status())), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 7, 0, 3, 0, 0, 0, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(get_status())), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 7, 0, 3, 0, 0, 0])), // Result Ready
        COBC(ACK),
        COBC(DATA(return_result(7, 3))),
        EDU(ACK),
        ACTION(Box::new(|bytes| {
            std::fs::File::create("tests/tmp/7.zip").unwrap().write(&bytes).unwrap();
        })),
        COBC(ACK),
        EDU(EOF),
        COBC(ACK),
        COBC(ACK),
    ];

    common::prepare_program("7");
    let (mut com, mut exec) = common::prepare_handles(packets, "7");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    std::process::Command::new("unzip")
        .current_dir("./tests/tmp")
        .arg("-o")
        .arg("7.zip")
        .status()?;

    assert_eq!(std::fs::read("tests/tmp/3")?, vec![0xde, 0xad]);
    assert!(std::fs::read("tests/tmp/7_3.log").is_ok());

    common::cleanup("7");
    Ok(())
}

/// Checks wether result files that are to large are truncated
#[test]
fn truncate_result() -> TestResult {
    let packets = vec![
        COBC(DATA(execute_program(8, 5, 5))), // Execute Program 8, Queue 5, Timeout 2s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(3000)),
        COBC(DATA(get_status())),
        EDU(ACK),
        EDU(DATA(vec![1, 8, 0, 5, 0, 0, 0, 0])),
        COBC(ACK),
    ];

    common::prepare_program("8");
    let (mut com, mut exec) = common::prepare_handles(packets, "8");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/8_5.zip")?.metadata()?.len() < 1_001_000);

    common::cleanup("8");
    Ok(())
}

#[test]
fn stopped_return() -> TestResult {
    let packets = vec![
        COBC(DATA(execute_program(9, 5, 3))),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(3000)),
        COBC(DATA(get_status())),
        EDU(ACK),
        EDU(DATA(vec![1, 9, 0, 5, 0, 0, 0, 0])),
        COBC(ACK),
        COBC(DATA(get_status())),
        EDU(ACK),
        EDU(DATA(vec![2, 9, 0, 5, 0, 0, 0])),
        COBC(ACK),
        COBC(DATA(return_result(9, 5))),
        EDU(ACK),
        ANY,
        COBC(ACK),
        ANY,
        COBC(STOP),
        COBC(DATA(get_status())),
        EDU(ACK),
        EDU(DATA(vec![2, 9, 0, 5, 0, 0, 0])),
        COBC(ACK),
    ];
    common::prepare_program("9");
    let (mut com, mut exec) = common::prepare_handles(packets, "9");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/9_5.zip").is_ok());

    common::cleanup("9");
    Ok(())
}

#[test]
fn no_result_ready() -> TestResult {
    let packets = vec![COBC(DATA(return_result(99, 0))), EDU(ACK), EDU(NACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "10");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("10");
    Ok(())
}

#[test]
fn result_is_not_deleted_after_corrupted_transfer() -> TestResult {
    let packets = vec![
        COBC(DATA(execute_program(50, 0, 3))),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(2000)),
        COBC(DATA(return_result(50, 0))),
        EDU(ACK),
        ANY,
        COBC(ACK),
        EDU(EOF),
        COBC(ACK),
        COBC(NACK)
    ];
    common::prepare_program("50");
    let (mut com, mut exec) = common::prepare_handles(packets, "50");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/50_0.zip").is_ok());

    common::cleanup("50");
    Ok(())
}
