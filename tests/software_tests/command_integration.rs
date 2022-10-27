use std::fs;
use std::io::{Read, Write};
use STS1_EDU_Scheduler::command::{self, CommandError};
use STS1_EDU_Scheduler::communication::{CSBIPacket::*, CommunicationError};

mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn store_archive() -> TestResult {
    // Define what should happen during communication. How this should look is defined in the PDD
    let packets = vec![
        COBC(DATA(vec![0x01, 0x00, 0x00])), // COBC sends Store Archive Command (0x01 -> Header, [0x00, 0x00] -> Program Id)
        EDU(ACK),                           // EDU acknowledges packet integrity
        COBC(DATA(fs::read("./tests/student_program.zip")?)), // COBC sends the archive
        EDU(ACK),                           // EDU acknowledges packet integrity
        COBC(EOF),                          // COBC signals end of packets
        EDU(ACK),                           // EDU signals successful Store Archive
    ];

    // Setup testing environment
    let (mut com, mut exec) = common::prepare_handles(packets, "0"); // construct handles for process_command

    // Run actual test
    command::handle_command(&mut com, &mut exec)?; // test the command processing

    // Check if all packets haven been sent/received
    assert!(com.is_complete());

    // Perform further checks
    assert_eq!(
        0,
        std::process::Command::new("diff") // check wether the archive was stored correctly
            .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/0"])
            .status()?
            .code()
            .unwrap()
    );

    // Cleanup testing environment
    common::cleanup("0");
    Ok(())
}

#[test]
fn execute_program_normal() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02])), // Execute Program ID 1, Queue ID 0, Timeout 2s
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("1");
    let (mut com, mut exec) = common::prepare_handles(packets, "1");

    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut res = String::new();
    std::fs::File::open("./archives/1/results/0")?.read_to_string(&mut res)?;

    assert_eq!(res.replace("\r", ""), *"Some test results\nWith multiple lines\n".to_string());

    common::cleanup("1");
    Ok(())
}

#[test]
fn execute_program_infinite() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x01])), // Execute Program ID 2, Queue ID 1, Timeout 1s
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("2");
    let (mut com, mut exec) = common::prepare_handles(packets, "2");

    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(1300));
    assert!(!exec.lock().unwrap().status_q.is_empty()?);

    common::cleanup("2");
    Ok(())
}

#[test]
fn stop_program() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x03, 0x00, 0x01, 0x00, 0x0a])), // Execute Program 3, Queue 1, Timeout 10s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_secs(1)),
        COBC(DATA(vec![0x03])),
        EDU(ACK),
        EDU(ACK),
    ];
    common::prepare_program("3");
    let (mut com, mut exec) = common::prepare_handles(packets, "3");

    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());
    assert!(!exec.lock().unwrap().status_q.is_empty()?);

    common::cleanup("3");
    Ok(())
}

#[test]
fn stopped_store() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x01, 0x00, 0x04])), // Store Archive with ID 1
        EDU(ACK),
        COBC(DATA(fs::read("./tests/student_program.zip")?)),
        EDU(ACK),
        COBC(DATA(vec![0, 1, 2, 3])),
        EDU(ACK),
        COBC(STOP),
    ];

    let (mut com, mut exec) = common::prepare_handles(packets, "4");

    let err = command::handle_command(&mut com, &mut exec).unwrap_err();
    assert!(matches!(err, CommandError::CommunicationError(CommunicationError::STOPCondition)));

    assert!(!std::path::Path::new("./archives/4").exists());

    common::cleanup("4");
    Ok(())
}

#[test]
fn get_status_none() -> TestResult {
    let packets = vec![COBC(DATA(vec![4])), EDU(ACK), EDU(DATA(vec![0])), COBC(ACK)];

    let (mut com, mut exec) = common::prepare_handles(packets, "5");
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    common::cleanup("5");
    Ok(())
}

#[test]
fn get_status_finished() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01])), // Execute Program 6, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 0, 6, 0, 0, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 0, 6, 0, 0])), // Result Ready
        COBC(ACK),
    ];

    common::prepare_program("6");
    let (mut com, mut exec) = common::prepare_handles(packets, "6");

    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    common::cleanup("6");
    Ok(())
}

#[test]
fn return_result() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x07, 0x00, 0x03, 0x00, 0x01])), // Execute Program 7, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 0, 7, 0, 3, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 0, 7, 0, 3])), // Result Ready
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

    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
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
        COBC(DATA(vec![2, 0, 8, 0, 5, 0, 2])), // Execute Program 8, Queue 5, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(2000)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 0, 8, 0, 5, 0])),
        COBC(ACK),
    ];

    common::prepare_program("8");
    let (mut com, mut exec) = common::prepare_handles(packets, "8");

    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    assert!(fs::File::open("./data/8_5.zip")?.metadata()?.len() < 1_001_000);

    common::cleanup("8");
    Ok(())
}

#[test]
fn stopped_return() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![2, 0, 9, 0, 5, 0, 2])),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(2000)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 0, 9, 0, 5, 0])),
        COBC(ACK),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![2, 0, 9, 0, 5])),
        COBC(ACK),
        COBC(DATA(vec![5])),
        EDU(ACK),
        ANY,
        COBC(ACK),
        ANY,
        COBC(STOP),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![2, 0, 9, 0, 5])),
        COBC(ACK),
    ];
    common::prepare_program("9");
    let (mut com, mut exec) = common::prepare_handles(packets, "9");

    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    command::handle_command(&mut com, &mut exec)?;
    let err = command::handle_command(&mut com, &mut exec).unwrap_err();
    assert!(matches!(err, CommandError::CommunicationError(CommunicationError::STOPCondition)));
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    assert!(fs::File::open("./data/9_5.zip").is_ok());

    common::cleanup("9");
    Ok(())
}

#[test]
fn no_result_ready() -> TestResult {
    let packets = vec![COBC(DATA(vec![5])), EDU(ACK), EDU(NACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "10");

    command::handle_command(&mut com, &mut exec).unwrap_err();
    assert!(com.is_complete());

    common::cleanup("10");
    Ok(())
}

#[test]
fn stop_no_running_program() -> TestResult {
    let packets = vec![COBC(DATA(vec![3])), EDU(ACK), EDU(ACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "11");
    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());
    Ok(())
}

#[test]
fn execute_missing_program() -> TestResult {
    let packets = vec![COBC(DATA(vec![2, 0, 0x0b, 0, 0, 0, 1])), EDU(ACK), EDU(NACK)];
    let (mut com, mut exec) = common::prepare_handles(packets, "12");

    command::handle_command(&mut com, &mut exec).unwrap_err();
    assert!(com.is_complete());

    common::cleanup("12");
    Ok(())
}

#[test]
fn invalid_packets_from_cobc() -> TestResult {
    let packets = vec![
        COBC(ACK),
        COBC(STOP),
        COBC(EOF),
        COBC(NACK),
        COBC(DATA(vec![1, 2])),
        EDU(NACK),
        COBC(DATA(vec![2, 0, 1])),
        EDU(NACK),
        COBC_INVALID(vec![0x8b, 0, 2, 0, 0, 0, 0, 1, 10]), // Invalid CRC
        EDU(NACK),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "13");

    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();
    command::handle_command(&mut com, &mut exec).unwrap_err();

    assert!(com.is_complete());

    common::cleanup("13");
    Ok(())
}

#[test]
fn invalid_crc() -> TestResult {
    let mut bytes = fs::read("./tests/student_program.zip")?;
    let packets = vec![
        COBC(DATA(vec![1, 0, 14])),
        EDU(ACK),
        COBC(DATA(bytes.drain(0..20).collect())),
        EDU(ACK),
        COBC_INVALID(vec![0x8b, 0, 5, 0, 0, 0, 0, 0, 0, 10, 10, 10]),
        EDU(NACK),
        COBC(DATA(bytes)),
        EDU(ACK),
        COBC(EOF),
        EDU(ACK),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "14");

    command::handle_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    assert_eq!(
        0,
        std::process::Command::new("diff") // check wether the archive was stored correctly
            .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/14"])
            .status()?
            .code()
            .unwrap()
    );

    common::cleanup("14");
    Ok(())
}
