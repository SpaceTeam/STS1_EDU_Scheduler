use crate::communication::CSBIPacket;
use crate::persist::FileQueue;
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;
use std::thread;
use std::time::Duration;
use std::process::Command;

use super::ProgramStatus;
use super::ResultId;
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

/// Executes a students program and starts a watchdog for it. The watchdog also creates entries in the
/// status and result queue found in `context`. The result, including logs, is packed into
/// `./data/{program_id}_{queue_id}`
///
/// * `context` The object containing the execution state
/// * `program_id` The name of the ./archives/ subfolder
/// * `queue_id` The first argument for the student program
/// * `timeout` The maxmimum time the student program shall execute. Will be rounded up to the nearest second
pub fn execute_program(
    context: &mut ExecutionContext,
    program_id: u16,
    queue_id: u16,
    timeout: Duration
) -> CommandResult {
    let _ = stop_program(context); // Ignore return value

    log::info!("Executing program {}:{}", program_id, queue_id);

    // TODO config setuid
    let output_file = File::create(format!("./data/{}_{}.log", program_id, queue_id))?;
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        detached: false,
        stdout: subprocess::Redirection::File(output_file),
        stderr: subprocess::Redirection::Merge,
        ..Default::default()
    };
    let mut student_process = subprocess::Popen::create(&["python", "main.py", &queue_id.to_string()], config)?;

    // Interthread communication
    let wd_flag = Arc::new(atomic::AtomicBool::new(true));
    let ec_flag = wd_flag.clone(); // clone before original is moved into thread
    let status_queue = context.status_q.clone();
    let result_queue = context.result_q.clone();

    // Watchdog thread
    let wd_handle = thread::spawn(move || {
        let mut exit_code = 255u8;
        let mut should_kill = true;

        // Loop over timeout in 1s steps
        for _ in 0..timeout.as_secs() {
            if let Some(status) = student_process.wait_timeout(Duration::from_secs(1)).unwrap() {
                // student program terminated itself
                if let subprocess::ExitStatus::Exited(n) = status {
                    exit_code = n as u8
                }
                should_kill = false;
                break;
            }
            if !wd_flag.load(atomic::Ordering::Relaxed) {
                // check if it should terminate
                break;
            }
        }

        if should_kill {
            log::warn!("Student Process timed out or is stopped");
            student_process.kill().unwrap();
            student_process.wait_timeout(Duration::from_millis(200)).unwrap().unwrap(); // Panic if not stopped
        }

        let rid = ResultId { program_id, queue_id };
        build_result_archive(rid).unwrap();

        let mut s_queue = status_queue.lock().unwrap();
        let mut r_queue = result_queue.lock().unwrap(); 
        s_queue.push(ProgramStatus { program_id, queue_id, exit_code }).unwrap();
        r_queue.push(rid).unwrap();

        wd_flag.store(false, atomic::Ordering::Relaxed);
    });

    context.thread_handle = Some(wd_handle);
    context.running_flag = Some(ec_flag);

    Ok(())
}

/// The function uses `tar` to create an uncompressed archive that includes the result file specified, as well as
/// the programs stdout/stderr and the schedulers log file. If any of the files is missing, the archive
/// is created without them.
fn build_result_archive(res: ResultId) -> Result<(), std::io::Error> {
    let res_path = format!("./archives/{}/results/{}", res.program_id, res.queue_id);
    let log_path = format!("./data/{}_{}.log", res.program_id, res.queue_id);
    let out_path = format!("./data/{}_{}.zip", res.program_id, res.queue_id);

    const MAXIMUM_FILE_SIZE: u64 = 1_000_000;
    truncate_to_size(&log_path, MAXIMUM_FILE_SIZE)?;
    truncate_to_size(&out_path, MAXIMUM_FILE_SIZE)?;
    truncate_to_size("log", MAXIMUM_FILE_SIZE)?;

    let _ = Command::new("zip")
        .arg(out_path)
        .arg("--junk-paths")
        .arg("log")
        .arg(res_path)
        .arg(log_path)
        .status();
    
    Ok(())
}

/// Truncates the file at `path` to the given size
fn truncate_to_size(path: &str, n_bytes: u64) -> Result<(), std::io::Error> {
    let file = std::fs::File::options().write(true).open(path)?;
    let size = file.metadata()?.len();
    if size > n_bytes {
        log::warn!("Truncating {} from {} bytes", path, size);
        file.set_len(n_bytes)?;
    }
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

/// The function returns a DATA packet that conforms to the Get Status specification in the PDD.
/// 
/// **Panics if no lock can be obtained on the queues.**
pub fn get_status(context: &mut ExecutionContext) -> Result<CSBIPacket, CommandError> {
    let mut s_queue = context.status_q.lock().unwrap();
    let mut r_queue = context.result_q.lock().unwrap();

    let s_empty = s_queue.is_empty()?;
    let r_empty = r_queue.is_empty()?;

    if s_empty && r_empty {
        Ok(CSBIPacket::DATA(vec![0]))
    }
    else if !s_empty {
        let mut v = vec![1];
        v.extend(s_queue.raw_pop()?);
        Ok(CSBIPacket::DATA(v))
    }
    else {
        let mut v = vec![2];
        v.extend(r_queue.raw_peek()?);
        Ok(CSBIPacket::DATA(v))
    }
}

/// Returns a byte vector containing the tar archive of the next element in the result queue.
/// It does **not** delete said element, as transmission might have been stopped/failed.
pub fn return_result(context: &ExecutionContext) -> Result<Vec<u8>, CommandError> {
    let mut r_queue = context.result_q.lock().unwrap();
    let res = r_queue.peek()?;
    let bytes = std::fs::read(format!("./data/{}_{}.zip", res.program_id, res.queue_id))?;
    Ok(bytes)
}

/// Deletes the result archive corresponding to the next element in the result queue and removes
/// that element from the queue.
pub fn delete_result(context: &mut ExecutionContext) -> CommandResult {
    let mut r_queue = context.result_q.lock().unwrap();
    let res = r_queue.pop()?;
    drop(r_queue); // Unlock Mutex
    
    let res_path = format!("./archives/{}/results/{}", res.program_id, res.queue_id);
    let log_path = format!("./data/{}_{}.log", res.program_id, res.queue_id);
    let out_path = format!("./data/{}_{}.zip", res.program_id, res.queue_id);
    let _ = std::fs::remove_file(res_path);
    let _ = std::fs::remove_file(log_path);
    let _ = std::fs::remove_file(out_path);

    Ok(())
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
