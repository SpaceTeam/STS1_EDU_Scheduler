use crate::software_tests::common;
use crate::software_tests::common::ComEvent::*;
use common::*;
use STS1_EDU_Scheduler::command::{self};
use STS1_EDU_Scheduler::communication::CEPPacket::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn stops_running_program() -> TestResult {
    let packets = vec![
        COBC(Data(execute_program(3, 1, 10))), // Execute Program 3, Queue 1, Timeout 10s
        EDU(Ack),
        EDU(Ack),
        SLEEP(std::time::Duration::from_secs(1)),
        COBC(Data(stop_program())),
        EDU(Ack),
        EDU(Ack),
        COBC(Data(get_status())),
        EDU(Ack),
        EDU(Data(vec![1, 3, 0, 1, 0, 0, 0, 255])),
        COBC(Ack),
    ];
    common::prepare_program("3");
    let (mut com, mut exec) = common::prepare_handles(packets, "3");

    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());

    common::cleanup("3");
    Ok(())
}

#[test]
fn stop_no_running_program() -> TestResult {
    let packets = vec![COBC(Data(stop_program())), EDU(Ack), EDU(Ack)];
    let (mut com, mut exec) = common::prepare_handles(packets, "11");
    command::handle_command(&mut com, &mut exec);
    assert!(com.is_complete());
    Ok(())
}
