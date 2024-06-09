use std::{
    error::Error, io::{Read, Write}, path::Path, process::{Child, ChildStdin, ChildStdout, Stdio}, time::Duration
};

use STS1_EDU_Scheduler::communication::{CEPPacket, CommunicationHandle};

fn main() {
    let scheduler_path =
        std::env::args().nth(1).expect("Pass in the directory containing the scheduler binary");

    let mut serial = SocatSerialPort::new(&format!("{scheduler_path}/virtualserial"));
    write_scheduler_config(&scheduler_path);
    let _scheduler = PoisonedChild(
        std::process::Command::new(format!("{scheduler_path}/STS1_EDU_Scheduler"))
            .current_dir(&scheduler_path)
            .spawn()
            .unwrap(),
    );

    loop {
        inquire_and_send_command(&mut serial, &scheduler_path).unwrap();
        println!("------------------------");
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub struct SocatSerialPort<T: Read, U: Write> {
    child: Child,
    stdout: T,
    stdin: U,
}

impl SocatSerialPort<ChildStdout, ChildStdin> {
    fn new(path: &str) -> Self {
        let mut child = std::process::Command::new("socat")
            .arg("stdio")
            .arg(format!("pty,raw,echo=0,link={},b921600,wait-slave", path))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        loop {
            if std::path::Path::new(path).exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take().unwrap();
        Self { child, stdout, stdin }
    }
}

fn write_scheduler_config(path: &str) {
    std::fs::write(
        format!("{path}/config.toml"),
        "
        uart = \"virtualserial\"
        baudrate = 921600
        heartbeat_pin = 34
    update_pin = 35
    heartbeat_freq = 10
    log_path = \"log\"
    ",
    )
    .unwrap();
}

const COMMANDS: &[&str] =
    &["StoreArchive", "ExecuteProgram", "StopProgram", "GetStatus", "ReturnResult", "UpdateTime"];

fn inquire_and_send_command(edu: &mut impl CommunicationHandle, path: &str) -> Result<(), Box<dyn Error>> {
    let mut select = inquire::Select::new("Select command", COMMANDS.to_vec());
    if Path::new(&format!("{path}/updatepin")).exists() {
        select.help_message = Some("Update Pin is high");
    }
    let command = select.prompt()?;

    match command {
        "StoreArchive" => {
            let archive = inquire::Text::new("Path to zipfile:").prompt()?;
            let program_id = inquire::Text::new("Program id (must be numerical):").prompt()?;
            let archive = std::fs::read(archive)?;

            edu.send_packet(&CEPPacket::Data(store_archive(program_id.parse()?)))?;
            edu.send_multi_packet(&archive)?;
            println!("Received {:?}", edu.receive_packet()?);
        }
        "ExecuteProgram" => {
            let program_id = inquire::Text::new("Program id:").prompt()?.parse()?;
            let timestamp = inquire::Text::new("Timestamp:").prompt()?.parse()?;
            let timeout =
                inquire::Text::new("Timeout (in seconds):").with_default("1").prompt()?.parse()?;

            edu.send_packet(&CEPPacket::Data(execute_program(program_id, timestamp, timeout)))?;
            println!("Received {:?}", edu.receive_packet()?);
        }
        "StopProgram" => {
            edu.send_packet(&CEPPacket::Data(stop_program()))?;
            println!("Received {:?}", edu.receive_packet()?);
        }
        "GetStatus" => {
            edu.send_packet(&CEPPacket::Data(get_status()))?;
            if let CEPPacket::Data(status) = edu.receive_packet()? {
                match status[0] {
                    0 => println!("No Event"),
                    1 => println!(
                        "Program Finished with ID: {} Timestamp: {} Exit Code: {}",
                        u16::from_le_bytes(status[1..3].try_into()?),
                        u32::from_le_bytes(status[3..7].try_into()?),
                        status[7]
                    ),
                    2 => println!(
                        "Result ready for ID: {} Timestamp: {}",
                        u16::from_le_bytes(status[1..3].try_into()?),
                        u32::from_le_bytes(status[3..7].try_into()?)
                    ),
                    3 => println!("Enable dosimeter"),
                    4 => println!("Disable dosimeter"),
                    n => println!("Unknown event {n}"),
                }
            }
        },
        "ReturnResult" => {
            let program_id = inquire::Text::new("Program id:").prompt()?.parse()?;
            let timestamp = inquire::Text::new("Timestamp:").prompt()?.parse()?;
            let result_path = inquire::Text::new("File path for returned result:").with_default("./result.tar").prompt()?;
            edu.send_packet(&CEPPacket::Data(return_result(program_id, timestamp)))?;
            match edu.receive_multi_packet() {
                Ok(data) => {
                    std::fs::write(result_path, data)?;
                    edu.send_packet(&CEPPacket::Ack)?;
                    println!("Wrote result to file");
                },
                Err(e) => println!("Received {:?}", e),
            }
        }
        _ => (),
    }

    Ok(())
}

impl<T: Read, U: Write> Read for SocatSerialPort<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stdout.read(buf)
    }
}

impl<T: Read, U: Write> Write for SocatSerialPort<T, U> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdin.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdin.flush()
    }
}

impl<T: Read, U: Write> Drop for SocatSerialPort<T, U> {
    fn drop(&mut self) {
        self.child.kill().unwrap();
    }
}

impl<T: Read, U: Write> CommunicationHandle for SocatSerialPort<T, U> {
    const INTEGRITY_ACK_TIMEOUT: Duration = Duration::MAX;
    const UNLIMITED_TIMEOUT: Duration = Duration::MAX;

    fn set_timeout(&mut self, _timeout: Duration) {}
}

struct PoisonedChild(pub Child);
impl Drop for PoisonedChild {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
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
