use clap::Parser;
use std::{
    error::Error,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Stdio},
    time::Duration,
};
use STS1_EDU_Scheduler::communication::{CEPPacket, CommunicationHandle};

#[derive(clap::Parser)]
enum Args {
    /// Simulate a serialport through socat, with the scheduler running on your host
    Simulate {
        #[arg(default_value = "target/release")]
        target_dir: PathBuf,
    },
    /// Connect serialport which has an EDU already running
    Serial {
        serialport: String,
        #[arg(long, short, default_value_t = 115200)]
        baudrate: u32,
    },
}

fn main() {
    let args = Args::parse();

    match args {
        Args::Simulate { target_dir } => {
            write_scheduler_config(&target_dir);
            let mut handle = SimulationContext::new(&target_dir.join("virtualserial"));
            inquire_loop(&mut handle);
        }
        Args::Serial { serialport, baudrate } => {
            let mut serial = serialport::new(serialport, baudrate).open().unwrap();
            inquire_loop(&mut serial);
        }
    };
}

fn inquire_loop(handle: &mut impl CommunicationHandle) -> ! {
    loop {
        inquire_and_send_command(handle).unwrap();
        println!("------------------------");
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub struct SimulationContext<T: Read, U: Write> {
    socat: Child,
    _scheduler: PoisonedChild,
    stdout: T,
    stdin: U,
}

impl SimulationContext<ChildStdout, ChildStdin> {
    fn new(path: &Path) -> Self {
        let mut child = std::process::Command::new("socat")
            .arg("stdio")
            .arg(format!("pty,raw,echo=0,link={},b921600,wait-slave", path.display()))
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

        let scheduler = PoisonedChild(
            std::process::Command::new("./STS1_EDU_Scheduler").current_dir(path).spawn().unwrap(),
        );

        let stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take().unwrap();
        Self { socat: child, _scheduler: scheduler, stdout, stdin }
    }
}

fn write_scheduler_config(path: &Path) {
    std::fs::write(
        path.join("config.toml"),
        "
        uart = \"virtualserial\"
        baudrate = 921600
        heartbeat_pin = 34
    update_pin = 35
    heartbeat_freq = 10
    log_path = \"log\"
    socket = \"/tmp/scheduler_socket\"
    ",
    )
    .unwrap();
}

const COMMANDS: &[&str] =
    &["StoreArchive", "ExecuteProgram", "StopProgram", "GetStatus", "ReturnResult", "UpdateTime"];

fn inquire_and_send_command(edu: &mut impl CommunicationHandle) -> Result<(), Box<dyn Error>> {
    let select = inquire::Select::new("Select command", COMMANDS.to_vec());
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
                match status.first().unwrap() {
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
        }
        "ReturnResult" => {
            let program_id = inquire::Text::new("Program id:").prompt()?.parse()?;
            let timestamp = inquire::Text::new("Timestamp:").prompt()?.parse()?;
            let result_path = inquire::Text::new("File path for returned result:")
                .with_default("./result")
                .prompt()?;
            edu.send_packet(&CEPPacket::Data(return_result(program_id, timestamp)))?;
            match edu.receive_multi_packet() {
                Ok(data) => {
                    std::fs::write(result_path, data)?;
                    edu.send_packet(&CEPPacket::Ack)?;
                    println!("Wrote result to file");
                }
                Err(e) => println!("Received {e:?}"),
            }
        }
        "UpdateTime" => {
            let actual = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let since_epoch = inquire::prompt_u32("Seconds since epoch (empty for current time):")
                .unwrap_or(actual as u32);

            edu.send_packet(&CEPPacket::Data(update_time(since_epoch)))?;
            println!("Received {:?}", edu.receive_packet()?);
        }
        c => unimplemented!("{c}"),
    }

    Ok(())
}

impl<T: Read, U: Write> Read for SimulationContext<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stdout.read(buf)
    }
}

impl<T: Read, U: Write> Write for SimulationContext<T, U> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdin.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdin.flush()
    }
}

impl<T: Read, U: Write> Drop for SimulationContext<T, U> {
    fn drop(&mut self) {
        self.socat.kill().unwrap();
    }
}

impl<T: Read, U: Write> CommunicationHandle for SimulationContext<T, U> {
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

#[must_use]
pub fn store_archive(program_id: u16) -> Vec<u8> {
    let mut vec = vec![1u8];
    vec.extend(program_id.to_le_bytes());
    vec
}

#[must_use]
pub fn execute_program(program_id: u16, timestamp: u32, timeout: u16) -> Vec<u8> {
    let mut vec = vec![2u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec.extend(timeout.to_le_bytes());
    vec
}

#[must_use]
pub fn stop_program() -> Vec<u8> {
    vec![3u8]
}

#[must_use]
pub fn get_status() -> Vec<u8> {
    vec![4u8]
}

#[must_use]
pub fn return_result(program_id: u16, timestamp: u32) -> Vec<u8> {
    let mut vec = vec![5u8];
    vec.extend(program_id.to_le_bytes());
    vec.extend(timestamp.to_le_bytes());
    vec
}

#[must_use]
pub fn update_time(since_epoch: u32) -> Vec<u8> {
    let mut vec = vec![6u8];
    vec.extend(since_epoch.to_le_bytes());
    vec
}
