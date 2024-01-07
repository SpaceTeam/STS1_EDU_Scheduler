use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn store_archive() -> TestResult {
    // Define what should happen during communication. How this should look is defined in the PDD
    let packets = vec![
        COBC(Data(vec![0x01, 0x00, 0x00])), // COBC sends Store Archive Command (0x01 -> Header, [0x00, 0x00] -> Program Id)
        EDU(Ack),                           // EDU acknowledges packet integrity
        COBC(Data(std::fs::read("./tests/student_program.zip")?)), // COBC sends the archive
        EDU(Ack),                           // EDU acknowledges packet integrity
        COBC(Eof),                          // COBC signals end of packets
        EDU(Ack),                           // EDU acknowledges Eof
        EDU(Ack),                           // EDU signals successful Store Archive
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
