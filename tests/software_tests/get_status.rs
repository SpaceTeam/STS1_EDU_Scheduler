use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;
mod common;
use common::ComEvent::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn get_status_none() -> TestResult {
    let packets = vec![COBC(DATA(vec![4])), EDU(ACK), EDU(DATA(vec![0])), COBC(ACK)];

    let (mut com, mut exec) = common::prepare_handles(packets, "5");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("5");
    Ok(())
}

#[test]
fn get_status_finished() -> TestResult {
    let packets = vec![
        COBC(DATA(vec![0x02, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00])), // Execute Program 6, Queue 0, Timeout 1s
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![1, 6, 0, 0, 0, 0])), // Program Finished
        COBC(ACK),
        COBC(DATA(vec![4])), // Get Status
        EDU(ACK),
        EDU(DATA(vec![2, 6, 0, 0, 0])), // Result Ready
        COBC(ACK),
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
        COBC(DATA(vec![2, 15, 0, 0, 0, 2, 0])),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 15, 0, 0, 0, 0])),
        COBC(ACK),
        COBC(DATA(vec![2, 15, 0, 0, 0, 2, 0])),
        EDU(ACK),
        EDU(ACK),
        SLEEP(std::time::Duration::from_millis(500)),
        COBC(DATA(vec![4])),
        EDU(ACK),
        EDU(DATA(vec![1, 15, 0, 0, 0, 0])),
        COBC(ACK),
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
