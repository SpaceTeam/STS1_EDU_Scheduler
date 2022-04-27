#![allow(clippy::collapsible_if)]

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path;
use std::sync::*;
use std::thread;
use std::time::Duration;
use crate::communication;

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

/// Parse a command coming from the COBC
///
/// `data_path` A path to the file containing the received command
///
/// Returns the resulting command with parameters or passes along the IO Error from file access
pub fn process_payload(data_path: path::PathBuf) -> Result<Command, std::io::Error> {
    todo!();
}

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
pub fn store_archive(folder: &str, bytes: &Vec<u8>) -> Result<(), Box<dyn Error>> {
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
                return Err(format!("Unzip returned with {:?}", status).into());
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
    pub sender: mpsc::Sender<bool>,
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
pub fn execute_program(context: &mut Option<ExecutionContext>, program_id: &str, queue_id: &str) -> Result<(), Box<dyn Error>> {
    let _ = stop_program(context); // Ignore return value

    log::info!("Executing program: {} with {}", program_id, queue_id);

    // TODO config setuid
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        ..Default::default()
    };
    let mut student_process = subprocess::Popen::create(&["python", "main.py", queue_id], config)?;

    // Interthread communication
    let (tx, rx): (mpsc::Sender<bool>, mpsc::Receiver<bool>) = mpsc::channel();
    let wd_flag = Arc::new(atomic::AtomicBool::new(true));
    let ec_flag = Arc::clone(&wd_flag); // clone before original is moved into thread

    // Watchdog thread
    let wd_handle = thread::spawn(move || {
        // TODO proper timeout
        for _ in 0..2 {
            if student_process.poll().is_some() { // student program terminated itself
                wd_flag.store(false, atomic::Ordering::Relaxed);
                return;
            }
            if rx.try_recv().is_ok() { // check if it should terminate
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
        log::info!("flag {}", wd_flag.load(atomic::Ordering::Relaxed));
    });

    *context = Some(ExecutionContext {sender: tx, thread_handle: wd_handle, running_flag: Arc::clone(&ec_flag)});

    Ok(())
}


/// Stops the currently running student program
/// 
/// * `context` The execution context of the student program (returns Err if context is None)
/// 
/// Returns Ok after terminating the student program of immediately if it is already stopped
/// 
/// **Panics if terminating takes too long**
pub fn stop_program(context: &mut Option<ExecutionContext>) -> Result<(), Box<dyn Error>> {
    if let Some(ec) = context {
        if ec.sender.send(true).is_ok() {
            log::warn!("Stopping running program"); // only is_ok if thread is still running
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

    Err("No program running".into())
}


/// Zips the results of the given program execution and sends the filepath to the communication module.
/// The results are taken from ./archives/program_id/results/queue_id
/// 
/// * `com_handle` The communication context, containing the needed sender
/// * `program_id` The programs folder name
/// * `queue_id` The name of the results subfolder
/// 
/// **Panics if the filepath can't be sent to the com module**
pub fn return_results(com_handle: &mut communication::CommunicationHandle, program_id: &str, queue_id: &str) -> Result<(), Box<dyn Error>> {
    todo!();
}

/// Places all program names found in the archive folder into a file, and passes it to the communication module.
/// 
/// * `com_handle` The communication context, containing the needed sender
pub fn list_files(com_handle: &mut communication::CommunicationHandle) -> Result<(), Box<dyn Error>> {
    todo!();
}

/// Updates the system time
/// 
/// * `epoch` Seconds since epoch (i32 works until Jan 2038)
pub fn update_time(epoch: i32) -> Result<(), Box<dyn Error>> {
    todo!();
}