use std::fs;
use std::io::{Read, Write};
use STS1_EDU_Scheduler::communication::{CSBIPacket::*, CommunicationError};
use STS1_EDU_Scheduler::command::{self, CommandError};

mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn store_archive() -> TestResult {
    let packets = vec![ // Define what should happen during communication
        COBC(DATA(vec![0x01, 0x00, 0x00])), // Store Archive with ID 0
        EDU(ACK),
        COBC(DATA(fs::read("./tests/student_program.zip")?)),
        EDU(ACK),
        COBC(EOF),
        EDU(ACK)
        ];
    let (mut com, mut exec) = common::prepare_handles(packets, "0"); // construct handles for process_command
    
    command::process_command(&mut com, &mut exec)?; // test the command processing
    assert!(com.is_complete()); // check if all packets were sent/received
    
    assert_eq!(0, std::process::Command::new("diff") // Check for correctness
    .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/0"])
    .status()?.code().unwrap());

    common::cleanup("0");
    Ok(())
}

#[test]
fn execute_program_normal() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02])), // Execute Program ID 1, Queue ID 0, Timeout 2s
        EDU(ACK),
        EDU(ACK)
    ];
    common::prepare_program("1");
    let (mut com, mut exec) = common::prepare_handles(packets, "1");

    command::process_command(&mut com, &mut exec)?;
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
        EDU(ACK)
    ];
    common::prepare_program("2");
    let (mut com, mut exec) = common::prepare_handles(packets, "2");
    
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(1300));
    assert!(!exec.status_q.lock().unwrap().is_empty()?);

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
        EDU(ACK)
    ];
    common::prepare_program("3");
    let (mut com, mut exec) = common::prepare_handles(packets, "3");

    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());
    assert!(!exec.status_q.lock().unwrap().is_empty()?);

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
        COBC(STOP)
    ];

    let (mut com, mut exec) = common::prepare_handles(packets, "4");

    let err = command::process_command(&mut com, &mut exec).unwrap_err();
    assert!(matches!(err, CommandError::CommunicationError(CommunicationError::STOPCondition)));

    assert!(!std::path::Path::new("./archives/4").exists());

    common::cleanup("4");
    Ok(())
}

#[test]
fn get_status_none() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![0])),
        COBC(ACK)
    ];

    let (mut com, mut exec) = common::prepare_handles(packets, "5");
    command::process_command(&mut com, &mut exec)?;
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
        COBC(ACK)
    ];

    common::prepare_program("6");
    let (mut com, mut exec) = common::prepare_handles(packets, "6");
    
    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
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
        COBC(NACK)
    ];

    common::prepare_program("7");
    let (mut com, mut exec) = common::prepare_handles(packets, "7");

    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
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
        SLEEP(std::time::Duration::from_millis(1000)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 0, 8, 0, 5, 0])),
        COBC(ACK)
    ];

    common::prepare_program("8");
    let (mut com, mut exec) = common::prepare_handles(packets, "8");

    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    assert!(fs::File::open("./data/8_5.zip")?.metadata()?.len() < 1_001_000);

    common::cleanup("8");
    Ok(())
}