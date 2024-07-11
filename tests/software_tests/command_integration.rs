use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use STS1_EDU_Scheduler::command;
use STS1_EDU_Scheduler::communication::CEPPacket::*;

#[test]
fn invalid_packets_from_cobc() {
    let packets = vec![
        Cobc(Data(vec![1, 2])),
        Edu(Ack),
        Edu(Nack),
        Cobc(Data(vec![2, 0, 1])),
        Edu(Ack),
        Edu(Nack),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "13");

    for _ in 0..2 {
        command::handle_command(&mut com, &mut exec);
    }

    assert!(com.is_complete());

    common::cleanup("13");
}

#[test]
#[should_panic]
fn ack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![Cobc(Ack)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn nack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![Cobc(Nack)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn eof_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![Cobc(Eof)], "99");
    command::handle_command(&mut com, &mut exec);
}
