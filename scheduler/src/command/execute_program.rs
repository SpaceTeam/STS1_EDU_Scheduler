use super::{CommandError, CommandResult, SyncExecutionContext};
use crate::{
    command::{
        check_length, terminate_student_program, Event, ProgramStatus, ResultId, RetryEvent,
    },
    communication::{CEPPacket, CommunicationHandle},
};
use anyhow::anyhow;
use simple_archive::Compression;
use std::{
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use subprocess::Popen;

const MAXIMUM_FILE_SIZE: usize = 1_000_000;

/// Executes a students program and starts a watchdog for it. The watchdog also creates entries in the
/// status and result queue found in `context`. The result, including logs, is packed into
/// `./data/{program_id}_{timestamp}`
pub fn execute_program(
    data: &[u8],
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, data, 9)?;

    let program_id = u16::from_le_bytes([data[1], data[2]]);
    let timestamp = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let timeout = Duration::from_secs(u16::from_le_bytes([data[7], data[8]]).into());
    log::info!("Executing Program {}:{} for {}s", program_id, timestamp, timeout.as_secs());

    terminate_student_program(exec).expect("to terminate a running program");

    let student_process = match create_student_process(program_id, timestamp) {
        Ok(p) => p,
        Err(e) => {
            com.send_packet(&CEPPacket::Nack)?;
            return Err(e);
        }
    };

    // WATCHDOG THREAD
    let mut wd_context = exec.clone();
    let wd_handle = std::thread::spawn(move || {
        let exit_code = supervise_process(student_process, timeout, &mut wd_context).unwrap_or(255);

        log::info!("Program {}:{} finished with {}", program_id, timestamp, exit_code);
        let sid = ProgramStatus { program_id, timestamp, exit_code };
        let rid = ResultId { program_id, timestamp };
        build_result_archive(rid).unwrap(); // create the tar file with result and log

        let mut context = wd_context.lock().unwrap();
        context.event_vec.push(RetryEvent::new(Event::Status(sid))).unwrap();
        context.event_vec.push(RetryEvent::new(Event::Result(rid))).unwrap();
        context.running_flag = false;
        context.update_pin.set_high();
        drop(context);
    });

    // After spawning the watchdog thread, store its handle and set flag
    let mut l_context = exec.lock().unwrap();
    l_context.thread_handle = Some(wd_handle);
    l_context.running_flag = true;
    drop(l_context);

    com.send_packet(&CEPPacket::Ack)?;
    Ok(())
}

/// This function creates and executes a student process. Its stdout/stderr is written into
/// `./data/[program_id]_[timestamp].log`
fn create_student_process(program_id: u16, timestamp: u32) -> Result<Popen, CommandError> {
    let program_path = format!("./archives/{program_id}/main.py");
    if !Path::new(&program_path).exists() {
        return Err(CommandError::ProtocolViolation(anyhow!("Could not find matching program")));
    }

    // TODO run the program from a student user (setuid)
    let output_file = std::fs::File::create(format!("./data/{program_id}_{timestamp}.log"))?; // will contain the stdout and stderr of the execute program
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{program_id}").into()),
        detached: false, // do not spawn as separate process
        stdout: subprocess::Redirection::File(output_file),
        stderr: subprocess::Redirection::Merge,
        ..Default::default()
    };

    let process = Popen::create(&["python", "main.py", &timestamp.to_string()], config)?;
    Ok(process)
}

/// A function intended to be run in a separate process, which checks every seconds if the given
/// timeout has passed or the process terminated itself. If it didnt, the process is killed.
fn supervise_process(
    mut process: Popen,
    timeout: Duration,
    exec: &mut SyncExecutionContext,
) -> Result<u8, ()> {
    if let Ok(code) = run_until_timeout(&mut process, timeout, exec) {
        Ok(code)
    } else {
        log::warn!("Student Process timed out or is stopped");
        process.kill().unwrap(); // send SIGKILL
        process
            .wait_timeout(Duration::from_millis(200)) // wait for it to do its magic
            .unwrap()
            .unwrap(); // Panic if not stopped
        Err(())
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
                #[allow(clippy::cast_possible_truncation)]
                return Ok(n as u8);
            }
            return Ok(0);
        }

        if !exec.lock().unwrap().running_flag {
            // if student program should be stopped
            break;
        }
    }

    Err(())
}

/// The function uses `tar` to create an uncompressed archive that includes the result file specified, as well as
/// the programs stdout/stderr and the schedulers log file. If any of the files is missing, the archive
/// is created without them.
fn build_result_archive(res: ResultId) -> Result<(), std::io::Error> {
    let out_path = PathBuf::from(&format!("./data/{res}"));
    let mut archive = simple_archive::Writer::new(std::fs::File::create(out_path)?);

    let res_path =
        PathBuf::from(format!("./archives/{}/results/{}", res.program_id, res.timestamp));
    let student_log_path = PathBuf::from(format!("./data/{res}.log"));
    let log_path = PathBuf::from("./log");

    add_to_archive_if_exists(&mut archive, &res.to_string(), &res_path, Compression::None)?;
    add_to_archive_if_exists(&mut archive, "student_log", &student_log_path, Compression::Zopfli)?;
    add_to_archive_if_exists(&mut archive, "log", &log_path, Compression::Zopfli)?;

    let _ = std::fs::remove_file(res_path);
    let _ = std::fs::remove_file(student_log_path);
    let _ = std::fs::OpenOptions::new().write(true).truncate(true).open(log_path);

    Ok(())
}

fn add_to_archive_if_exists<T: Write>(
    archive: &mut simple_archive::Writer<T>,
    name: &str,
    path: impl AsRef<Path>,
    compression: simple_archive::Compression,
) -> std::io::Result<()> {
    match std::fs::read(path) {
        Ok(mut data) => {
            data.truncate(MAXIMUM_FILE_SIZE);
            archive.append_data(name, &data, compression)?;
            Ok(())
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
