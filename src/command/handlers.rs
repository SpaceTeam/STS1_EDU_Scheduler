use subprocess::Popen;

use crate::communication::CSBIPacket;
use crate::communication::CommunicationHandle;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::thread;
use std::time::Duration;

use super::ProgramStatus;
use super::ResultId;
use super::{CommandError, CommandResult, SyncExecutionContext};

const COM_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::new(2, 0);

/// This function implements the Store Archive command, including the reception of the archive itself
pub fn store_archive(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    _exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 3)?;
    com.send_packet(CSBIPacket::ACK)?;

    let id = u16::from_le_bytes([data[1], data[2]]).to_string();
    log::info!("Storing Archive {}", id);

    let bytes = com.receive_multi_packet(&COM_TIMEOUT_DURATION, || false)?; // !! TODO !!
    unpack_archive(id, bytes)?;

    com.send_packet(CSBIPacket::ACK)?;
    Ok(())
}

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
pub fn unpack_archive(folder: String, bytes: Vec<u8>) -> CommandResult {
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
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 7)?;
    com.send_packet(CSBIPacket::ACK)?;

    let program_id = u16::from_le_bytes([data[1], data[2]]);
    let queue_id = u16::from_le_bytes([data[3], data[4]]);
    let timeout = Duration::from_secs(u16::from_le_bytes([data[5], data[6]]).into());
    log::info!("Executing Program {}:{} for {}s", program_id, queue_id, timeout.as_secs());

    terminate_student_program(exec).expect("to terminate a running program");

    let student_process = create_student_process(program_id, queue_id)?;

    // WATCHDOG THREAD
    let mut wd_context = exec.clone();
    let wd_handle = thread::spawn(move || {
        let exit_code = match supervise_process(student_process, timeout, &mut wd_context) {
            Ok(code) => code,
            Err(()) => 255,
        };

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
    let mut l_context = exec.lock().unwrap();
    l_context.thread_handle = Some(wd_handle);
    l_context.running_flag = true;
    drop(l_context);

    com.send_packet(CSBIPacket::ACK)?;
    Ok(())
}

fn create_student_process(program_id: u16, queue_id: u16) -> Result<Popen, CommandError> {
    // TODO run the program from a student user (setuid)
    let output_file = File::create(format!("./data/{}_{}.log", program_id, queue_id))?; // will contain the stdout and stderr of the execute program
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        detached: false, // do not spawn as separate process
        stdout: subprocess::Redirection::File(output_file),
        stderr: subprocess::Redirection::Merge,
        ..Default::default()
    };

    let process = Popen::create(&["python", "main.py", &queue_id.to_string()], config)?;
    Ok(process)
}

fn supervise_process(
    mut process: Popen,
    timeout: Duration,
    exec: &mut SyncExecutionContext,
) -> Result<u8, ()> {
    match run_until_timeout(&mut process, timeout, exec) {
        Ok(code) => Ok(code),
        Err(()) => {
            log::warn!("Student Process timed out or is stopped");
            process.kill().unwrap(); // send SIGKILL
            process
                .wait_timeout(Duration::from_millis(200)) // wait for it to do its magic
                .unwrap()
                .unwrap(); // Panic if not stopped
            Err(())
        }
    }
}

/// This function allows the program to run for timeout (rounded to seconds)
/// If the program terminates, it exit code is returned
/// If it times out or the running flag is reset, an Err is returned instead
fn run_until_timeout(
    process: &mut Popen,
    timeout: Duration,
    exec: &mut SyncExecutionContext,
) -> Result<u8, ()> {
    // Loop over timeout in 1s steps
    for _ in 0..timeout.as_secs() {
        if let Some(status) = process // if student program terminates with exit code
            .wait_timeout(Duration::from_secs(1))
            .unwrap()
        {
            if let subprocess::ExitStatus::Exited(n) = status {
                return Ok(n as u8);
            } else {
                return Ok(0);
            }
        }

        if !exec.lock().unwrap().running_flag {
            // if student program should be stopped
            break;
        }
    }

    Err(())
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
    let _ = truncate_to_size("log", MAXIMUM_FILE_SIZE);

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
pub fn stop_program(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 1)?;
    com.send_packet(CSBIPacket::ACK)?;

    terminate_student_program(exec).expect("to terminate student program");

    com.send_packet(CSBIPacket::ACK)?;
    Ok(())
}

fn terminate_student_program(exec: &mut SyncExecutionContext) -> CommandResult {
    let mut con = exec.lock().unwrap();
    if !con.is_running() {
        return Ok(());
    }
    con.running_flag = false; // Signal watchdog thread to terminate
    drop(con); // Release mutex

    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100)); // Sensible amount?
        let con = exec.lock().unwrap();
        if con.thread_handle.as_ref().unwrap().is_finished() {
            return Ok(());
        }
    }

    Err(CommandError::SystemError("Supervisor thread did not finish".into()))
}

/// The function returns a DATA packet that conforms to the Get Status specification in the PDD.
///
/// **Panics if no lock can be obtained on the queues.**
pub fn get_status(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 1)?;
    com.send_packet(CSBIPacket::ACK)?;

    let mut con = exec.lock().unwrap();

    let s_empty = con.status_q.is_empty()?;
    let r_empty = con.result_q.is_empty()?;

    match (s_empty, r_empty) {
        (false, _) => {
            log::info!("Sending program exit code");
            let mut v = vec![1];
            v.extend(con.status_q.raw_pop()?);
            if !con.has_data_ready()? {
                con.update_pin.set_low();
            }
            com.send_packet(CSBIPacket::DATA(v))?;
        }
        (true, false) => {
            log::info!("Sending result-ready");
            let mut v = vec![2];
            v.extend(con.result_q.raw_peek()?);
            com.send_packet(CSBIPacket::DATA(v))?;
        }
        _ => {
            log::info!("Nothing to report");
            com.send_packet(CSBIPacket::DATA(vec![0]))?;
        }
    };

    com.receive_packet(&COM_TIMEOUT_DURATION)?; // throw away ACK
    Ok(())
}

/// Returns a byte vector containing the tar archive of the next element in the result queue.
/// It does **not** delete said element, as transmission might have been stopped/failed.
pub fn return_result(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 1)?;
    com.send_packet(CSBIPacket::ACK)?;

    let mut con = exec.lock().unwrap();
    let res = con.result_q.peek()?;
    drop(con);

    let bytes = std::fs::read(format!("./data/{}_{}.zip", res.program_id, res.queue_id))?;
    log::info!("Returning result for {}:{}", res.program_id, res.queue_id);
    com.send_multi_packet(bytes, &COM_TIMEOUT_DURATION)?;

    let response = com.receive_packet(&COM_TIMEOUT_DURATION)?;
    if response == CSBIPacket::ACK {
        delete_result(exec)?;
    } else {
        log::error!("COBC did not acknowledge result");
    }

    Ok(())
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
pub fn update_time(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    _exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 5)?;
    com.send_packet(CSBIPacket::ACK)?;

    let time = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);
    set_system_time(time)?;

    com.send_packet(CSBIPacket::ACK)?;
    Ok(())
}

fn set_system_time(s_since_epoch: i32) -> CommandResult {
    let exit_status = Command::new("date").arg("-s").arg(format!("@{}", s_since_epoch)).status()?;
    if !exit_status.success() {
        return Err(CommandError::SystemError("date utility failed".into()));
    }

    Ok(())
}

fn check_length(vec: &Vec<u8>, n: usize) -> Result<(), CommandError> {
    if vec.len() != n {
        log::error!("Command came with {} bytes, should have {}", vec.len(), n);
        Err(CommandError::InvalidCommError)
    } else {
        Ok(())
    }
}
