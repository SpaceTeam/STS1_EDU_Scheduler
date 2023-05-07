use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CSBIPacket::*;
mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn store_archive() -> TestResult {
    // Define what should happen during communication. How this should look is defined in the PDD
    let packets = vec![
        COBC(DATA(vec![0x01, 0x00, 0x00])), // COBC sends Store Archive Command (0x01 -> Header, [0x00, 0x00] -> Program Id)
        EDU(ACK),                           // EDU acknowledges packet integrity
        COBC(DATA(std::fs::read("./tests/student_program.zip")?)), // COBC sends the archive
        EDU(ACK),                           // EDU acknowledges packet integrity
        COBC(EOF),                          // COBC signals end of packets
        EDU(ACK),                           // EDU signals successful Store Archive
    ];

    // Setup testing environment
    let (mut com, mut exec) = common::prepare_handles(packets, "0"); // construct handles for process_command

    // Run actual test
    command::handle_command(&mut com, &mut exec); // test the command processing

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
fn stopped_store() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x01, 0x04, 0x00])), // Store Archive with ID 0
        EDU(ACK),
        COBC(DATA(std::fs::read("./tests/student_program.zip")?)),
        EDU(ACK),
        COBC(DATA(vec![0, 1, 2, 3])),
        EDU(ACK),
        COBC(STOP),
    ];

    let (mut com, mut exec) = common::prepare_handles(packets, "4");

    command::handle_command(&mut com, &mut exec);

    assert!(!std::path::Path::new("./archives/4").exists());

    common::cleanup("4");
    Ok(())
}

#[test]
fn invalid_crc() -> TestResult {
    let mut bytes = std::fs::read("./tests/student_program.zip")?;
    let packets = vec![
        COBC(DATA(vec![1, 14, 0])),
        EDU(ACK),
        COBC(DATA(bytes.drain(0..20).collect())),
        EDU(ACK),
        COBC_INVALID(vec![0x8b, 5, 0, 0, 0, 0, 0, 0, 0, 10, 10, 10]),
        EDU(NACK),
        COBC(DATA(bytes)),
        EDU(ACK),
        COBC(EOF),
        EDU(ACK),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "14");

    command::handle_command(&mut com, &mut exec);
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
