#![allow(clippy::collapsible_if)]

use crate::communication::{self, CommunicationError};
use crate::communication::{CSBIPacket, CommunicationHandle};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::CommandEnvs;
use std::sync::*;
use std::thread;
use std::time::Duration;

/// An enum storing a COBC command with its parameters
#[derive(Debug)]
pub enum Command {
    StoreArchive(String, Vec<u8>),
    ExecuteProgram(String, String),
    StopProgram,
    ReturnResults(String, String),
    ListFiles,
    UpdateTime(i32),
}

type CommandResult = Result<(), CommandError>;

const COM_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::new(2, 0);

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn process_command(com: &mut impl CommunicationHandle, exec: &mut Option<ExecutionContext>) -> CommandResult {
    // Preprocess
    let packet = com.receive_packet(&COM_TIMEOUT_DURATION)?;
    let data = if let CSBIPacket::DATA(bytes) = packet {
        bytes
    }
    else {
        return Err(CommandError::InvalidCommError); // Did not start with a data packet
    };

    match data[0] {
        0x01 => {
            com.send_packet(CSBIPacket::ACK)?;
            let id = data[1].to_string();
            let bytes = com.receive_multi_packet(&COM_TIMEOUT_DURATION, || {false})?; // !! TODO !!
            store_archive(id, bytes)?;
            com.send_packet(CSBIPacket::ACK)?;
        },
        _ => {
            return Err(CommandError::InvalidCommError);
        }
    };
    
    return Ok(());
}

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
pub fn store_archive(folder: String, bytes: Vec<u8>) -> CommandResult {
    log::info!("Storing archive {}", folder);

    // Store bytes into temporary file
    let zip_path = format!("./data/{}.zip", folder);
    let mut zip_file = File::create(&zip_path)?;
    zip_file.write_all(&bytes)?;
    zip_file.sync_all()?;

    let exit_status = subprocess::Exec::cmd("unzip")
        .arg("-o") // overwrite silently
        .arg(&zip_path)
        .arg("-d") // target directory
        .arg(format!("./archives/{}", folder))
        .join();

    std::fs::remove_file(zip_path)?;

    match exit_status {
        Ok(status) => {
            if !status.success() {
                return Err(CommandError::SystemError("unzip failed".into()));
            }
        }
        Err(err) => {
            return Err(err.into());
        }
    }

    Ok(())
}

/// This struct is used to store the relevant handles for when a student program is executed
pub struct ExecutionContext {
    pub thread_handle: thread::JoinHandle<()>,
    pub running_flag: Arc<atomic::AtomicBool>,
}

impl ExecutionContext {
    pub fn is_running(&self) -> bool {
        return self.running_flag.load(atomic::Ordering::Relaxed);
    }
}

/// Executes a students program and starts a watchdog for it
///
/// * `program_id` The name of the ./archives/ subfolder
/// * `queue_id` The first argument for the student program
pub fn execute_program(
    context: &mut Option<ExecutionContext>,
    program_id: &str,
    queue_id: &str,
) -> CommandResult {
    let _ = stop_program(context); // Ignore return value

    log::info!("Executing program {}:{}", program_id, queue_id);

    // TODO config setuid
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        ..Default::default()
    };
    let mut student_process = subprocess::Popen::create(&["python", "main.py", queue_id], config)?;

    // Interthread communication
    let wd_flag = Arc::new(atomic::AtomicBool::new(true));
    let ec_flag = Arc::clone(&wd_flag); // clone before original is moved into thread

    // Watchdog thread
    let wd_handle = thread::spawn(move || {
        // TODO proper timeout
        for _ in 0..2 {
            if student_process.poll().is_some() {
                // student program terminated itself
                wd_flag.store(false, atomic::Ordering::Relaxed);
                return;
            }
            if !wd_flag.load(atomic::Ordering::Relaxed) {
                // check if it should terminate
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }

        student_process.terminate().unwrap(); // SIGTERM
        if student_process
            .wait_timeout(Duration::from_millis(100))
            .unwrap()
            .is_none()
        {
            log::warn!("Program not responding to SIGTERM, proceeding with SIGKILL");
            student_process.kill().unwrap(); // SIGKILL if still running
        }
        wd_flag.store(false, atomic::Ordering::Relaxed);
    });

    *context = Some(ExecutionContext {
        thread_handle: wd_handle,
        running_flag: Arc::clone(&ec_flag),
    });

    Ok(())
}

/// Stops the currently running student program
///
/// * `context` The execution context of the student program (returns Err if context is None)
///
/// Returns Ok after terminating the student program of immediately if it is already stopped
///
/// **Panics if terminating takes too long**
pub fn stop_program(context: &mut Option<ExecutionContext>) -> CommandResult {
    if let Some(ec) = context {
        if ec.running_flag.load(atomic::Ordering::Relaxed) {
            log::warn!("Stopping running program");
            ec.running_flag.store(false, atomic::Ordering::Relaxed);
            // wait until it is stopped
            for _ in 0..120 {
                if !ec.is_running() {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(10));
            }

            panic!("Could not stop student process");
        }
    }

    Err(CommandError::SystemError("Program not running".into())) // TODO Should this be an error?
}

/// Zips the results of the given program execution and sends the filepath to the communication module.
/// The results are taken from ./archives/program_id/results/queue_id
///
/// * `com_handle` The communication context, containing the needed sender
/// * `program_id` The programs folder name
/// * `queue_id` The results subfolder name
///
/// **Panics if the filepath can't be sent to the com module**
pub fn return_results(program_id: &str, queue_id: &str) -> CommandResult {
    log::info!("Returning Results for {}:{}", program_id, queue_id);

    let zip_path = std::path::PathBuf::from(format!("./data/{}{}.zip", program_id, queue_id));
    let res_path =
        std::path::PathBuf::from(format!("./archives/{}/results/{}", program_id, queue_id));

    if !res_path.exists() {
        log::warn!("Results folder does not exist");
    }

    subprocess::Exec::cmd("zip")
        .arg("-r")
        .arg(zip_path.as_os_str())
        .arg("log")
        .arg(res_path.as_os_str())
        .join()?;

    if !zip_path.exists() {
        return Err(CommandError::SystemError("Archive not created".into()));
    }


    todo!(); // depends on send multi packet

    let _ = std::fs::remove_dir_all(res_path);
    std::fs::File::create("log")?.set_len(0)?;
}


/// Updates the system time
///
/// * `epoch` Seconds since epoch (i32 works until Jan 2038)
pub fn update_time(epoch: i32) -> CommandResult {
    let exit_status = subprocess::Exec::cmd("date")
        .arg("-s")
        .arg(format!("@{}", epoch))
        .join()?;

    if !exit_status.success() {
        return Err(CommandError::SystemError("date utility failed".into()));
    }
    
    Ok(())
}


#[derive(Debug)]
pub enum CommandError {
    /// Propagates an error from the communication module
    CommunicationError(CommunicationError),
    /// Signals that something has gone wrong while using a system tool (e.g. unzip)
    SystemError(Box<dyn std::error::Error>),
    /// Signals that packets that are not useful right now were received
    InvalidCommError
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<subprocess::PopenError> for CommandError {
    fn from(e: subprocess::PopenError) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<CommunicationError> for CommandError {
    fn from(e: CommunicationError) -> Self {
        CommandError::CommunicationError(e)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CommandError {}