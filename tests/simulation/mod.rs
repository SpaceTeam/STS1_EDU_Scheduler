mod command_execution;
mod full_run;
mod logging;
mod socket;
mod timeout;

use std::{
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Stdio},
    time::Duration,
};
use STS1_EDU_Scheduler::communication::{CEPPacket, CommunicationError, CommunicationHandle};

pub struct SimulationComHandle<T: Read, U: Write> {
    cobc_in: T,
    cobc_out: U,
}

impl SimulationComHandle<ChildStdout, ChildStdin> {
    fn with_socat_proc(unique: &str) -> (Self, PoisonedChild) {
        let mut proc = std::process::Command::new("socat")
            .arg("stdio")
            .arg(format!("pty,raw,echo=0,link=/tmp/ttySTS1-{},b921600,wait-slave", unique))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        loop {
            if std::path::Path::new(&format!("/tmp/ttySTS1-{unique}")).exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        (
            Self { cobc_in: proc.stdout.take().unwrap(), cobc_out: proc.stdin.take().unwrap() },
            PoisonedChild(proc),
        )
    }
}

impl<T: Read, U: Write> Read for SimulationComHandle<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cobc_in.read(buf)
    }
}

impl<T: Read, U: Write> Write for SimulationComHandle<T, U> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cobc_out.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.cobc_out.flush()
    }
}

impl<T: Read, U: Write> CommunicationHandle for SimulationComHandle<T, U> {
    const INTEGRITY_ACK_TIMEOUT: Duration = Duration::MAX;
    const UNLIMITED_TIMEOUT: Duration = Duration::MAX;

    fn set_timeout(&mut self, _timeout: std::time::Duration) {}
}

fn get_config_str(unique: &str) -> String {
    format!(
        "
    uart = \"/tmp/ttySTS1-{unique}\"
    baudrate = 921600
    heartbeat_pin = 34
    update_pin = 35
    heartbeat_freq = 10
    log_path = \"log\"
    socket = \"/tmp/STS1_EDU_Scheduler_SIM_{unique}\"
    "
    )
}

/// A simple wrapper that ensures child processes are killed when dropped
struct PoisonedChild(pub Child);
impl Drop for PoisonedChild {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

fn start_scheduler(unique: &str) -> Result<PoisonedChild, std::io::Error> {
    let test_dir = format!("./tests/tmp/{}", unique);
    let scheduler_bin = std::fs::canonicalize("./target/release/STS1_EDU_Scheduler")?;
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir)?;
    std::fs::write(format!("{}/config.toml", &test_dir), get_config_str(unique))?;

    let scheduler =
        std::process::Command::new(scheduler_bin).current_dir(test_dir).spawn().unwrap();

    Ok(PoisonedChild(scheduler))
}

pub fn simulate_test_store_archive(
    com: &mut impl CommunicationHandle,
    program_id: u16,
) -> Result<(), CommunicationError> {
    let archive = std::fs::read("tests/student_program.zip").unwrap();
    com.send_packet(&CEPPacket::Data(store_archive(program_id)))?;
    com.send_multi_packet(&archive)?;
    com.await_ack(Duration::MAX)?;

    Ok(())
}

pub fn simulate_execute_program(
    com: &mut impl CommunicationHandle,
    program_id: u16,
    timestamp: u32,
    timeout: u16,
) -> Result<(), CommunicationError> {
    com.send_packet(&CEPPacket::Data(execute_program(program_id, timestamp, timeout)))?;
    com.await_ack(Duration::MAX)?;

    Ok(())
}

pub fn simulate_get_status(
    com: &mut impl CommunicationHandle,
) -> Result<Vec<u8>, CommunicationError> {
    com.send_packet(&CEPPacket::Data(get_status()))?;
    let response = com.receive_packet()?;

    if let CEPPacket::Data(data) = response {
        Ok(data)
    } else {
        Err(CommunicationError::PacketInvalidError)
    }
}

pub fn simulate_return_result(
    com: &mut impl CommunicationHandle,
    program_id: u16,
    timestamp: u32,
) -> Result<Vec<u8>, CommunicationError> {
    com.send_packet(&CEPPacket::Data(return_result(program_id, timestamp)))?;
    let data = com.receive_multi_packet()?;

    Ok(data)
}

pub fn store_archive(program_id: u16) -> Vec<u8> {
    let mut vec = vec![1u8];
    vec.extend(program_id.to_le_bytes());
    vec
}

pub fn execute_program(program_id: u16, timestamp: u32, timeout: u16) -> Vec<u8> {
    let mut vec = vec![2u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec.extend(timeout.to_le_bytes());
    vec
}

pub fn stop_program() -> Vec<u8> {
    vec![3u8]
}

pub fn get_status() -> Vec<u8> {
    vec![4u8]
}

pub fn return_result(program_id: u16, timestamp: u32) -> Vec<u8> {
    let mut vec = vec![5u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec
}
