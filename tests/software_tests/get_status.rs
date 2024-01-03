use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn get_status_none() -> TestResult {
    let packets = vec![COBC(Data(vec![4])), EDU(Ack), EDU(Data(vec![0])), COBC(Ack)];

    let (mut com, mut exec) = common::prepare_handles(packets, "5");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("5");
    Ok(())
}

#[test]
fn get_status_finished() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(6, 0, 1))), // Execute Program 6, Queue 0, Timeout 1s
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(Data(vec![4])), // Get Status
        EDU(Ack),
        EDU(Data(vec![1, 6, 0, 0, 0, 0, 0, 0])), // Program Finished
        COBC(Ack),
        COBC(Data(vec![4])), // Get Status
        EDU(Ack),
        EDU(Data(vec![2, 6, 0, 0, 0, 0, 0])), // Result Ready
        COBC(Ack),
    ];

    common::prepare_program("6");
    let (mut com, mut exec) = common::prepare_handles(packets, "6");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("6");
    Ok(())
}

#[test]
fn get_status_priority_for_status() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(15, 0, 2))),
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(Data(get_status())),
        EDU(Ack),
        EDU(Data(vec![1, 15, 0, 0, 0, 0, 0, 0])),
        COBC(Ack),
        COBC(Data(execute_program(15, 0, 2))),
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(Data(get_status())),
        EDU(Ack),
        EDU(Data(vec![1, 15, 0, 0, 0, 0, 0, 0])),
        COBC(Ack),
    ];
    common::prepare_program("15");
    let (mut com, mut exec) = common::prepare_handles(packets, "15");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("15");
    Ok(())
}
