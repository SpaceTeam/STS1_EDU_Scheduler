use crate::communication::CSBIPacket;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::thread;
use std::time::Duration;

use super::ProgramStatus;
use super::ResultId;
use super::{CommandError, CommandResult, SyncExecutionContext};

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
pub fn store_archive(folder: String, bytes: Vec<u8>) -> CommandResult {
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

    // Remove the temporary file, even if unzip failed
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
    context: &mut SyncExecutionContext,
    program_id: u16,
    queue_id: u16,
    timeout: Duration,
) -> CommandResult {
    let _ = stop_program(context); // Ignore return value

    // TODO config setuid
    let output_file = File::create(format!("./data/{}_{}.log", program_id, queue_id))?; // will contain the stdout and stderr of the execute program
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        detached: false, // do not spawn as separate process
        stdout: subprocess::Redirection::File(output_file),
        stderr: subprocess::Redirection::Merge,
        ..Default::default()
    };
    let mut student_process =
        subprocess::Popen::create(&["python", "main.py", &queue_id.to_string()], config)?;

    // create a reference for the watchdog thread
    let wd_context = context.clone();

    // Watchdog thread
    let wd_handle = thread::spawn(move || {
        let mut exit_code = 255u8; // the programs exit code (overwritten if applicable)
        let mut should_kill_student_program = true; // set to false if it terminates itself

        // Loop over timeout in 1s steps
        for _ in 0..timeout.as_secs() {
            if let Some(status) = student_process // if student program terminates with exit code
                .wait_timeout(Duration::from_secs(1))
                .unwrap()
            {
                if let subprocess::ExitStatus::Exited(n) = status {
                    // if it terminated itself
                    exit_code = n as u8
                }
                should_kill_student_program = false;
                break; // leave timeout loop
            }

            if !wd_context.lock().unwrap().running_flag {
                // if student program should be stopped
                break;
            }
        }

        if should_kill_student_program {
            log::warn!("Student Process timed out or is stopped");
            student_process.kill().unwrap(); // send SIGKILL
            student_process
                .wait_timeout(Duration::from_millis(200)) // wait for it to do its magic
                .unwrap()
                .unwrap(); // Panic if not stopped
        }

        log::info!("Program {}:{} finished with {}", program_id, queue_id, exit_code);
        let rid = ResultId { program_id, queue_id };
        build_result_archive(rid).unwrap(); // create the zip file with result and log

        let mut context = wd_context.lock().unwrap();
        context.status_q.push(ProgramStatus { program_id, queue_id, exit_code }).unwrap();
        context.result_q.push(rid).unwrap();
        context.running_flag = false;
        context.update_pin.set_high(); // Set EDU_Update pin
        drop(context);
    });

    // After spawning the watchdog thread, store its handle and set flag
    let mut l_context = context.lock().unwrap();
    l_context.thread_handle = Some(wd_handle);
    l_context.running_flag = true;

    Ok(())
}

/// The function uses `zip` to create an uncompressed archive that includes the result file specified, as well as
/// the programs stdout/stderr and the schedulers log file. If any of the files is missing, the archive
/// is created without them.
fn build_result_archive(res: ResultId) -> Result<(), std::io::Error> {
    let res_path = format!("./archives/{}/results/{}", res.program_id, res.queue_id);
    let log_path = format!("./data/{}_{}.log", res.program_id, res.queue_id);
    let out_path = format!("./data/{}_{}.zip", res.program_id, res.queue_id);

    const MAXIMUM_FILE_SIZE: u64 = 1_000_000;
    let _ = truncate_to_size(&log_path, MAXIMUM_FILE_SIZE);
    let _ = truncate_to_size(&res_path, MAXIMUM_FILE_SIZE);
    let _ = truncate_to_size("lpog", MAXIMUM_FILE_SIZE);

    let _ = Command::new("zip")
        .arg("-0")
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
        file.sync_all()?;
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
pub fn stop_program(context: &mut SyncExecutionContext) -> CommandResult {
    let mut con = context.lock().unwrap();
    if !con.is_running() {
        return Ok(());
    }
    con.running_flag = false; // Signal watchdog thread to terminate
    drop(con); // Release mutex

    std::thread::sleep(Duration::from_millis(2000)); // Sensible amount?

    assert!(
        // Panic if the watchdog thread is not finished
        context.lock().unwrap().thread_handle.as_ref().unwrap().is_finished(),
        "Watchdog thread did not finish in time"
    );

    Ok(())
}

/// The function returns a DATA packet that conforms to the Get Status specification in the PDD.
///
/// **Panics if no lock can be obtained on the queues.**
pub fn get_status(context: &mut SyncExecutionContext) -> Result<CSBIPacket, CommandError> {
    let mut con = context.lock().unwrap();

    let s_empty = con.status_q.is_empty()?;
    let r_empty = con.result_q.is_empty()?;

    if s_empty && r_empty {
        log::info!("Nothing to report");
        Ok(CSBIPacket::DATA(vec![0]))
    } else if !s_empty {
        log::info!("Sending program exit code");
        let mut v = vec![1];
        v.extend(con.status_q.raw_pop()?);
        if !con.has_data_ready()? {
            con.update_pin.set_low();
        }
        Ok(CSBIPacket::DATA(v))
    } else {
        log::info!("Sending result-ready");
        let mut v = vec![2];
        v.extend(con.result_q.raw_peek()?);
        Ok(CSBIPacket::DATA(v))
    }
}

/// Returns a byte vector containing the tar archive of the next element in the result queue.
/// It does **not** delete said element, as transmission might have been stopped/failed.
pub fn return_result(context: &SyncExecutionContext) -> Result<Vec<u8>, CommandError> {
    let mut con = context.lock().unwrap();
    let res = con.result_q.peek()?;
    drop(con);

    let bytes = std::fs::read(format!("./data/{}_{}.zip", res.program_id, res.queue_id))?;
    log::info!("Returning result for {}:{}", res.program_id, res.queue_id);
    Ok(bytes)
}

/// Deletes the result archive corresponding to the next element in the result queue and removes
/// that element from the queue.
pub fn delete_result(context: &mut SyncExecutionContext) -> CommandResult {
    let mut con = context.lock().unwrap();
    let res = con.result_q.pop()?;
    if !con.has_data_ready()? {
        con.update_pin.set_low();
    }
    drop(con); // Unlock Mutex

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
    let exit_status = Command::new("date").arg("-s").arg(format!("@{}", epoch)).status()?;

    if !exit_status.success() {
        return Err(CommandError::SystemError("date utility failed".into()));
    }

    Ok(())
}
