use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn get_status_none() {
    let packets = vec![Cobc(Data(vec![4])), Edu(Ack), Edu(Data(vec![0])), Cobc(Ack)];

    let (mut com, mut exec) = common::prepare_handles(packets, "5");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("5");
}

#[test]
fn get_status_finished() {
    let packets = vec![
        Cobc(Data(execute_program(6, 0, 1))), // Execute Program 6, Queue 0, Timeout 1s
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(500)),
        Cobc(Data(vec![4])), // Get Status
        Edu(Ack),
        Edu(Data(vec![1, 6, 0, 0, 0, 0, 0, 0])), // Program Finished
        Cobc(Ack),
        Cobc(Data(vec![4])), // Get Status
        Edu(Ack),
        Edu(Data(vec![2, 6, 0, 0, 0, 0, 0])), // Result Ready
        Cobc(Ack),
    ];

    common::prepare_program("6");
    let (mut com, mut exec) = common::prepare_handles(packets, "6");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("6");
}

#[test]
fn get_status_priority_for_status() {
    let packets = vec![
        Cobc(Data(execute_program(15, 0, 2))),
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(500)),
        Cobc(Data(get_status())),
        Edu(Ack),
        Edu(Data(vec![1, 15, 0, 0, 0, 0, 0, 0])),
        Cobc(Ack),
        Cobc(Data(execute_program(15, 0, 2))),
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_millis(500)),
        Cobc(Data(get_status())),
        Edu(Ack),
        Edu(Data(vec![1, 15, 0, 0, 0, 0, 0, 0])),
        Cobc(Ack),
    ];
    common::prepare_program("15");
    let (mut com, mut exec) = common::prepare_handles(packets, "15");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("15");
}
