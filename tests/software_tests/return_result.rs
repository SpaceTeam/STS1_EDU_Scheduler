use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn returns_result_correctly() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(7, 3, 1))), // Execute Program 7, Queue 0, Timeout 1s
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(Data(get_status())), // Get Status
        EDU(Ack),
        EDU(Data(vec![1, 7, 0, 3, 0, 0, 0, 0])), // Program Finished
        COBC(Ack),
        COBC(Data(get_status())), // Get Status
        EDU(Ack),
        EDU(Data(vec![2, 7, 0, 3, 0, 0, 0])), // Result Ready
        COBC(Ack),
        COBC(Data(return_result(7, 3))),
        EDU(Ack),
        ACTION(Box::new(|packet| {
            let bytes = packet.clone().serialize();
            std::fs::write("tests/tmp/7.tar", &bytes[3..bytes.len() - 4]).unwrap();
        })),
        COBC(Ack),
        EDU(Eof),
        COBC(Ack),
        COBC(Ack),
    ];

    common::prepare_program("7");
    let (mut com, mut exec) = common::prepare_handles(packets, "7");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    let _ = std::fs::create_dir("./tests/tmp/7_unpack");
    std::process::Command::new("tar")
        .current_dir("./tests/tmp/7_unpack")
        .arg("xf")
        .arg("../7.tar")
        .status()?;

    assert_eq!(std::fs::read("./tests/tmp/7_unpack/3")?, vec![0xde, 0xad]);
    assert!(std::fs::read("./tests/tmp/7_unpack/7_3.log").is_ok());

    common::cleanup("7");
    Ok(())
}

/// Checks wether result files that are to large are truncated
#[test]
fn truncate_result() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(8, 5, 5))), // Execute Program 8, Queue 5, Timeout 2s
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(3000)),
        COBC(Data(get_status())),
        EDU(Ack),
        EDU(Data(vec![1, 8, 0, 5, 0, 0, 0, 0])),
        COBC(Ack),
    ];

    common::prepare_program("8");
    let (mut com, mut exec) = common::prepare_handles(packets, "8");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/8_5.tar")?.metadata()?.len() < 1_005_000);

    common::cleanup("8");
    Ok(())
}

#[test]
fn no_result_ready() -> TestResult {
    let packets = vec![COBC(Data(return_result(99, 0))), EDU(Ack), EDU(Nack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "10");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("10");
    Ok(())
}

#[test]
fn result_is_not_deleted_after_corrupted_transfer() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(50, 0, 3))),
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(2000)),
        COBC(Data(return_result(50, 0))),
        EDU(Ack),
        ANY,
        COBC(Ack),
        EDU(Eof),
        COBC(Ack),
        COBC(Nack),
    ];
    common::prepare_program("50");
    let (mut com, mut exec) = common::prepare_handles(packets, "50");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/50_0.tar").is_ok());

    common::cleanup("50");
    Ok(())
}
