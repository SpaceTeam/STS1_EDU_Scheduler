use std::fs;
use std::io::Read;
use STS1_EDU_Scheduler::communication::CSBIPacket::*;
use STS1_EDU_Scheduler::command;

mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn store_archive() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x01, 0x00, 0x01])),
        EDU(ACK),
        COBC(DATA(fs::read("./tests/student_program.zip")?)),
        EDU(ACK),
        COBC(EOF),
        EDU(ACK)
        ];
    let (mut com, mut exec) = common::prepare_handles(packets);
    
    command::process_command(&mut com, &mut exec)?;
    assert!(com.is_complete());
    
    assert_eq!(0, std::process::Command::new("diff")
    .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/0"])
    .status()?.code().unwrap());

    std::fs::remove_dir_all("./archives/0")?;
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

    let mut res = String::new();    
    std::fs::File::open("./archives/normal/results/0")?.read_to_string(&mut res)?;

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
    
    command::process_command(&mut com, &mut exec);
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

    command::process_command(&mut com, &mut exec);
    assert!(com.is_complete());

    std::fs::remove_dir_all("./archives/3")?;
    todo!("Check execution history entry");
    Ok(())
}