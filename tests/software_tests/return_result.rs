use std::io::Write;

use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;
mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn return_result() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x07, 0x00, 0x03, 0x00, 0x01, 0x00])), // Execute Program 7, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 7, 0, 3, 0, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 7, 0, 3, 0])), // Result Ready
        COBC(ACK),
        COBC(DATA(vec![5])),
        EDU(ACK),
        ACTION(Box::new(|bytes| {
            std::fs::File::create("tests/tmp/7.zip").unwrap().write(&bytes).unwrap();
        })),
        COBC(ACK),
        EDU(EOF),
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

#[test]
fn truncate_result() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![2, 8, 0, 5, 0, 5, 0])), // Execute Program 8, Queue 5, Timeout 2s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(3000)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 8, 0, 5, 0, 0])),
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
        COBC(DATA(vec![2, 9, 0, 5, 0, 3, 0])),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(3000)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 9, 0, 5, 0, 0])),
        COBC(ACK),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![2, 9, 0, 5, 0])),
        COBC(ACK),
        COBC(DATA(vec![5])),
        EDU(ACK),
        ANY,
        COBC(ACK),
        ANY,
        COBC(STOP),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![2, 9, 0, 5, 0])),
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
    let packets = vec![COBC(DATA(vec![5])), EDU(ACK), EDU(NACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "10");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("10");
    Ok(())
}
