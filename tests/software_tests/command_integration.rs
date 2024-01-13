use STS1_EDU_Scheduler::command;
use STS1_EDU_Scheduler::communication::CEPPacket::*;

use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_packets_from_cobc() -> TestResult {
    let packets = vec![
        COBC(Data(vec![1, 2])),
        EDU(Ack),
        EDU(Nack),
        COBC(Data(vec![2, 0, 1])),
        EDU(Ack),
        EDU(Nack),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "13");

    for _ in 0..2 {
        command::handle_command(&mut com, &mut exec);
    }

    assert!(com.is_complete());

    common::cleanup("13");
    Ok(())
}

#[test]
#[should_panic]
fn ack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(Ack)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn nack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(Nack)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn eof_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(Eof)], "99");
    command::handle_command(&mut com, &mut exec);
}
