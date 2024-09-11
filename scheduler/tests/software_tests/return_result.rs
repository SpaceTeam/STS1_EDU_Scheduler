use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use simple_archive::Entry;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn returns_result_correctly() -> TestResult {
    let packets = vec![
        Cobc(Data(execute_program(7, 3, 1))), // Execute Program 7, Queue 0, Timeout 1s
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(500)),
        Cobc(Data(get_status())), // Get Status
        Edu(Ack),
        Edu(Data(vec![1, 7, 0, 3, 0, 0, 0, 0])), // Program Finished
        Cobc(Ack),
        Cobc(Data(get_status())), // Get Status
        Edu(Ack),
        Edu(Data(vec![2, 7, 0, 3, 0, 0, 0])), // Result Ready
        Cobc(Ack),
        Cobc(Data(return_result(7, 3))),
        Edu(Ack),
        Action(Box::new(|packet| {
            let bytes = packet.clone().serialize();
            std::fs::write("tests/tmp/7_3", &bytes[3..bytes.len() - 4]).unwrap();
        })),
        Cobc(Ack),
        Edu(Eof),
        Cobc(Ack),
        Cobc(Ack),
    ];

    common::prepare_program("7");
    let (mut com, mut exec) = common::prepare_handles(packets, "7");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    let results = simple_archive::Reader::new(std::fs::File::open("tests/tmp/7_3")?)
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    dbg!(&results);
    assert!(results.contains(&Entry { path: "7_3".to_string(), data: vec![0xde, 0xad] }));
    assert!(results.iter().any(|e| e.path == "student_log"));

    common::cleanup("7");
    Ok(())
}

/// Checks wether result files that are to large are truncated
#[test]
fn truncate_result() -> TestResult {
    let packets = vec![
        Cobc(Data(execute_program(8, 5, 5))), // Execute Program 8, Queue 5, Timeout 2s
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(3000)),
        Cobc(Data(get_status())),
        Edu(Ack),
        Edu(Data(vec![1, 8, 0, 5, 0, 0, 0, 0])),
        Cobc(Ack),
    ];

    common::prepare_program("8");
    let (mut com, mut exec) = common::prepare_handles(packets, "8");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/8_5")?.metadata()?.len() < 1_005_000);

    common::cleanup("8");
    Ok(())
}

#[test]
fn no_result_ready() {
    let packets = vec![Cobc(Data(return_result(99, 0))), Edu(Ack), Edu(Nack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "10");

    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("10");
}

#[test]
fn result_is_not_deleted_after_corrupted_transfer() {
    let packets = vec![
        Cobc(Data(execute_program(50, 0, 3))),
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(2000)),
        Cobc(Data(return_result(50, 0))),
        Edu(Ack),
        Any,
        Cobc(Ack),
        Edu(Eof),
        Cobc(Ack),
        Cobc(Nack),
    ];
    common::prepare_program("50");
    let (mut com, mut exec) = common::prepare_handles(packets, "50");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    assert!(std::fs::File::open("./data/50_0").is_ok());

    common::cleanup("50");
}
