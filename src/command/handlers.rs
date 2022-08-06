use crate::communication::CSBIPacket;
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;
use std::thread;
use std::time::Duration;
use std::process::Command;

use super::{CommandResult, CommandError, ExecutionContext};

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

    let exit_status = Command::new("unzip")
        .arg("-o") // overwrite silently
        .arg(&zip_path)
        .arg("-d") // target directory
        .arg(format!("./archives/{}", folder))
        .status();

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

/// Executes a students program and starts a watchdog for it
///
/// * `program_id` The name of the ./archives/ subfolder
/// * `queue_id` The first argument for the student program
pub fn execute_program(
    context: &mut ExecutionContext,
    program_id: &str,
    queue_id: &str,
    timeout: &Duration
) -> CommandResult {
    let _ = stop_program(context); // Ignore return value

    log::info!("Executing program {}:{}", program_id, queue_id);

    // TODO config setuid
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        detached: false,
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

    context.thread_handle = Some(wd_handle);
    context.running_flag = Some(ec_flag);

    Ok(())
}

/// Stops the currently running student program
///
/// * `context` The execution context of the student program (returns Err if context is None)
///
/// Returns Ok after terminating the student program or immediately if it is already stopped
///
/// **Panics if terminating takes too long**
pub fn stop_program(context: &mut ExecutionContext) -> CommandResult {
    if context.is_running() {
        let flag = context.running_flag.as_ref().unwrap();
        let handle = context.thread_handle.as_ref().unwrap();
        flag.store(false, atomic::Ordering::Relaxed);
        for _ in 0..120 {
            if handle.is_finished() {
                return Ok(())
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("Could not stop student program");
    }

    Ok(())
}

pub fn get_status() -> Result<CSBIPacket, CommandError> {
    todo!();
}

pub fn return_result() -> Result<Vec<u8>, CommandError> {
    todo!();
}

pub fn delete_result() -> CommandResult {
    todo!();
}


/// Updates the system time
///
/// * `epoch` Seconds since epoch (i32 works until Jan 2038)
pub fn update_time(epoch: i32) -> CommandResult {
    let exit_status = Command::new("date")
        .arg("-s")
        .arg(format!("@{}", epoch))
        .status()?;

    if !exit_status.success() {
        return Err(CommandError::SystemError("date utility failed".into()));
    }
    
    Ok(())
}
