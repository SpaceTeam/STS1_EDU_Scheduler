mod command_execution;
mod logging;

use std::{
    fmt::format,
    io::{Read, Write},
    process::{Child, Stdio},
};

use crate::software_tests::common::*;
use STS1_EDU_Scheduler::communication::CEPPacket;

fn get_config_str(unique: &str) -> String {
    format!(
        "
    uart = \"/tmp/ttySTS1-{}\"
    baudrate = 921600
    heartbeat_pin = 34
    update_pin = 35
    heartbeat_freq = 10
    log_path = \"log\"
    ",
        unique
    )
}

pub fn start_scheduler(unique: &str) -> Result<(Child, Child), std::io::Error> {
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

    let scheduler =
        std::process::Command::new(scheduler_bin).current_dir(test_dir).spawn().unwrap();

    Ok((scheduler, serial_port))
}

pub fn receive_ack(reader: &mut impl std::io::Read) -> Result<(), std::io::Error> {
    let mut buffer = [0; 1];
    reader.read_exact(&mut buffer).unwrap();

    if buffer[0] == CEPPacket::ACK.serialize()[0] {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("received {:#x} instead of ACK", buffer[0]),
        ))
    }
}

pub fn simulate_test_store_archive(
    cobc_in: &mut impl std::io::Read,
    cobc_out: &mut impl std::io::Write,
    program_id: u16,
) -> std::io::Result<()> {
    let archive = std::fs::read("tests/student_program.zip")?;
    cobc_out.write_all(&CEPPacket::DATA(store_archive(program_id)).serialize())?;
    receive_ack(cobc_in)?;
    cobc_out.write_all(&CEPPacket::DATA(archive).serialize())?;
    receive_ack(cobc_in)?;
    cobc_out.write_all(&CEPPacket::EOF.serialize())?;
    receive_ack(cobc_in)?;
    receive_ack(cobc_in)?;

    Ok(())
}

pub fn simulate_execute_program(
    cobc_in: &mut impl std::io::Read,
    cobc_out: &mut impl std::io::Write,
    program_id: u16,
    timestamp: u32,
    timeout: u16,
) -> std::io::Result<()> {
    cobc_out
        .write_all(&CEPPacket::DATA(execute_program(program_id, timestamp, timeout)).serialize())?;
    receive_ack(cobc_in)?;
    receive_ack(cobc_in)?;
    Ok(())
}

pub fn simulate_return_result(
    cobc_in: &mut impl std::io::Read,
    cobc_out: &mut impl std::io::Write,
    program_id: u16,
    timestamp: u32,
) -> std::io::Result<Vec<u8>> {
    cobc_out.write_all(&CEPPacket::DATA(return_result(program_id, timestamp)).serialize())?;
    receive_ack(cobc_in)?;

    let data = read_multi_data_packets(cobc_in, cobc_out)?;
    Ok(data)
}

/// Reads a data packet from input and returns the data content (does not check CRC)
pub fn read_data_packet(input: &mut impl std::io::Read, data: &mut Vec<u8>) -> std::io::Result<()> {
    let mut header = [0; 3];
    input.read_exact(&mut header)?;
    if header[0] != 0x8B {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Expected data header (0x8B), received {:#04x}", header[0]),
        ));
    }

    let data_len = u16::from_le_bytes([header[1], header[2]]);
    input.take((data_len + 4).into()).read_to_end(data)?;

    Ok(())
}

/// Reads a multi packet round without checking the CRC and returns the concatenated contents
pub fn read_multi_data_packets(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
) -> std::io::Result<Vec<u8>> {
    let mut eof_byte = [0; 1];
    let mut data = Vec::new();
    loop {
        read_data_packet(input, &mut data)?;
        output.write_all(&CEPPacket::ACK.serialize())?;

        input.read_exact(&mut eof_byte)?;
        if eof_byte[0] == CEPPacket::EOF.serialize()[0] {
            break;
        }
    }

    output.write_all(&CEPPacket::ACK.serialize())?;
    Ok(data)
}
