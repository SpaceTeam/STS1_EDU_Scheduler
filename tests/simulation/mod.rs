mod logging;
mod command_execution;

use std::{io::Write, process::{Child, Stdio}};

use STS1_EDU_Scheduler::communication::CEPPacket;
use crate::software_tests::common::*;

fn get_config_str(unique: &str) -> String {
    format!("
    uart = \"/tmp/ttySTS1-{}\"
    baudrate = 921600
    heartbeat_pin = 34
    update_pin = 35
    heartbeat_freq = 10
    log_path = \"log\"
    ", unique)
}

pub fn start_scheduler(unique: &str) -> Result<(Child, Child), std::io::Error>{
    let test_dir = format!("./tests/tmp/{}", unique);
    let scheduler_bin = std::fs::canonicalize("./target/release/STS1_EDU_Scheduler")?;
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir)?;
    std::fs::write(format!("{}/config.toml", &test_dir), get_config_str(unique))?;

    let serial_port = std::process::Command::new("socat")
        .arg("stdio")
        .arg(format!("pty,raw,echo=0,link=/tmp/ttySTS1-{},b921600,wait-slave", unique))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    let scheduler = std::process::Command::new(scheduler_bin)
        .current_dir(test_dir)
        .spawn().unwrap();

    Ok((scheduler, serial_port))
} 

pub fn receive_ack(reader: &mut impl std::io::Read) -> Result<(), std::io::Error> {
    let mut buffer = [0; 1];
    reader.read_exact(&mut buffer).unwrap();
    
    if buffer[0] == CEPPacket::ACK.serialize()[0] {
        Ok(())
    }
    else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, format!("received {:#x} instead of ACK", buffer[0])))
    }
}

pub fn simulate_test_store_archive(cobc_in: &mut impl std::io::Read, cobc_out: &mut impl std::io::Write) -> std::io::Result<()> {
    let archive = std::fs::read("tests/student_program.zip")?;
    cobc_out.write_all(&CEPPacket::DATA(store_archive(1)).serialize())?;
    receive_ack(cobc_in)?;
    cobc_out.write_all(&CEPPacket::DATA(archive).serialize())?;
    cobc_out.write_all(&CEPPacket::EOF.serialize())?;
    receive_ack(cobc_in)?;
    receive_ack(cobc_in)?;

    Ok(())
}