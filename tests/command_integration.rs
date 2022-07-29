use std::fs;
use STS1_EDU_Scheduler::communication::CSBIPacket::*;
use STS1_EDU_Scheduler::command;

mod common;
use common::ComEvent::*;

#[test]
fn store_archive() -> Result <(), Box<dyn std::error::Error>> {
    let packets = vec![
        COBC(DATA(vec![0x01, 0x00, 0x01])),
        EDU(ACK),
        COBC(DATA(fs::read("./tests/student_program.zip")?)),
        EDU(ACK),
        COBC(EOF),
        EDU(ACK)
        ];
    let mut com = common::TestCom::new(packets);
    let mut exec: Option<command::ExecutionContext> = None;
    
    command::process_command(&mut com, &mut exec)?;
    
    assert_eq!(0, std::process::Command::new("diff")
    .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/0"])
    .status()?.code().unwrap());

    std::fs::remove_dir_all("./archives/0")?;
    Ok(())
}