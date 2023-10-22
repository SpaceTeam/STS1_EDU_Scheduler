use std::fs;
use std::io::{Read, Write};
use STS1_EDU_Scheduler::command::{self, CommandError};
use STS1_EDU_Scheduler::communication::{CEPPacket::*, CommunicationError};

use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_packets_from_cobc() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![1, 2])),
        EDU(NACK),
        COBC(DATA(vec![2, 0, 1])),
        EDU(NACK),
        COBC_INVALID(vec![0x8b, 2, 0, 0, 0, 0, 0, 1, 10]), // Invalid CRC
        EDU(NACK),
    ];
    let (mut com, mut exec) = common::prepare_handles(packets, "13");

    for _ in 0..3 {
        command::handle_command(&mut com, &mut exec);
    }

    assert!(com.is_complete());

    common::cleanup("13");
    Ok(())
}

#[test]
#[should_panic]
fn ack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(ACK)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn nack_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(NACK)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn eof_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(EOF)], "99");
    command::handle_command(&mut com, &mut exec);
}

#[test]
#[should_panic]
fn stop_on_start_panic() {
    let (mut com, mut exec) = common::prepare_handles(vec![COBC(STOP)], "99");
    command::handle_command(&mut com, &mut exec);
}
