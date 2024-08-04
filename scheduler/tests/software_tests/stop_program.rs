use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

#[test]
fn stops_running_program() {
    let packets = vec![
        Cobc(Data(execute_program(3, 1, 10))), // Execute Program 3, Queue 1, Timeout 10s
        Edu(Ack),
        Edu(Ack),
        Sleep(std::time::Duration::from_secs(1)),
        Cobc(Data(stop_program())),
        Edu(Ack),
        Edu(Ack),
        Cobc(Data(get_status())),
        Edu(Ack),
        Edu(Data(vec![1, 3, 0, 1, 0, 0, 0, 255])),
        Cobc(Ack),
    ];
    common::prepare_program("3");
    let (mut com, mut exec) = common::prepare_handles(packets, "3");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("3");
}

#[test]
fn stop_no_running_program() {
    let packets = vec![Cobc(Data(stop_program())), Edu(Ack), Edu(Ack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "11");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());
}
