use std::io::{Write, Read};

mod common;
use STS1_EDU_Scheduler::communication::CEPPacket;
use common::*;

#[test]
fn logfile_is_created() -> Result<(), std::io::Error> {
    let (mut scheduler, _) = common::start_scheduler("log_created")?;
    std::thread::sleep(std::time::Duration::from_millis(400));
    scheduler.kill().unwrap();

    assert!(std::path::Path::new("./tests/tmp/log_created/log").exists());
    Ok(())
}

#[test]
fn simulate_store_archive() -> Result<(), std::io::Error> {
    let (mut scheduler, mut serial_port) = common::start_scheduler("simulate_store_archive")?;
    let mut cobc_in = serial_port.stdout.take().unwrap();
    let mut cobc_out = serial_port.stdin.take().unwrap();

    let archive = std::fs::read("tests/student_program.zip")?;
    cobc_out.write_all(&CEPPacket::DATA(store_archive(1)).serialize())?;
    receive_ack(&mut cobc_in)?;
    cobc_out.write_all(&CEPPacket::DATA(archive).serialize())?;
    cobc_out.write_all(&CEPPacket::EOF.serialize())?;
    receive_ack(&mut cobc_in)?;
    receive_ack(&mut cobc_in)?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    assert_eq!(
        0,
        std::process::Command::new("diff") // check wether the archive was stored correctly
            .args(["-yq", "--strip-trailing-cr", "tests/test_data", "tests/tmp/simulate_store_archive/archives/1"])
            .status()?
            .code()
            .unwrap()
    );

    scheduler.kill()?;
    Ok(())
}