use std::fs;
use std::io::Read;
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
    let (mut com, mut exec) = common::prepare_handles(packets); // construct handles for process_command
    
    command::process_command(&mut com, &mut exec)?; // test the command processing
    assert!(com.is_complete()); // check if all packets were sent/received
    
    assert_eq!(0, std::process::Command::new("diff") // Check for correctness
    .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/0"])
    .status()?.code().unwrap());

    std::fs::remove_dir_all("./archives/0")?; // Cleanup
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
    let (mut com, mut exec) = common::prepare_handles(packets);

    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut res = String::new();    
    std::fs::File::open("./archives/1/results/0")?.read_to_string(&mut res)?;

    assert_eq!(res.replace("\r", ""), *"Some test results\nWith multiple lines\n".to_string());

    std::fs::remove_dir_all("./archives/1")?;
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
    let (mut com, mut exec) = common::prepare_handles(packets);
    
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::thread::sleep(std::time::Duration::from_millis(1300));

    std::fs::remove_dir_all("./archives/2")?;
    todo!("Check execution history entry");
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
    let (mut com, mut exec) = common::prepare_handles(packets);

    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    std::fs::remove_dir_all("./archives/3")?;
    todo!("Check execution history entry");
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

    let (mut com, mut exec) = common::prepare_handles(packets);

    let err = command::process_command(&mut com, &mut exec).unwrap_err();
    assert!(matches!(err, CommandError::CommunicationError(CommunicationError::STOPCondition)));

    assert!(!std::path::Path::new("./archives/4").exists());

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

    let (mut com, mut exec) = common::prepare_handles(packets);
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());

    Ok(())
}

#[test]
fn get_status_finished() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01])), // Execute Program 5, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 0, 5, 0, 0, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 0, 5, 0, 0])), // Result Ready
        COBC(ACK)
    ];

    common::prepare_program("5");
    let (mut com, mut exec) = common::prepare_handles(packets);
    
    command::process_command(&mut com, &mut exec)?;
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());
    
    std::fs::remove_dir_all("./archives/5")?;
    Ok(())
}
