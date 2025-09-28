use super::{CommandError, CommandResult, SyncExecutionContext};
use crate::{
    command::{
        Event, ProgramStatus, ResultId, RetryEvent, check_length, terminate_student_program,
    },
    communication::{CEPPacket, CommunicationHandle},
};
use anyhow::anyhow;
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};
use subprocess::Popen;
use zopfli::Options;

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

        log::info!("Program {program_id}:{timestamp} finished with {exit_code}");
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
        log::warn!("Student Process timed out or should be stopped");
        process.terminate().unwrap();
        if process.wait_timeout(Duration::from_millis(200)).unwrap().is_none() {
            log::warn!("Student Process did not react to SIGTERM, killing...");
            process.kill().unwrap();
        }
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

const RESULT_SIZE_LIMIT: u64 = 1_000_000;

fn build_result_archive(res: ResultId) -> Result<(), std::io::Error> {
    let result_dir =
        Path::new("archives").join(res.program_id.to_string()).join(res.timestamp.to_string());
    let log = Path::new("log");
    let student_log = Path::new("data").join(res.to_string() + ".log");

    let mut entries = Vec::new();

    if let Ok(size) = directory_file_size(&result_dir)
        && size < RESULT_SIZE_LIMIT
        && let Ok(files) = list_files(&result_dir)
    {
        let result_entries = files.into_iter().filter_map(|f| create_cpio_entry(f).ok());
        entries.extend(result_entries);
    } else {
        log::warn!(
            "Result directory for {res} cannot be read or exceeds {RESULT_SIZE_LIMIT} bytes"
        );
    }

    if let Ok(log) = create_compressed_cpio_entry(log) {
        entries.push(log);
    }
    if let Ok(student_log) = create_compressed_cpio_entry(&student_log) {
        entries.push(student_log);
    }

    let output_path = Path::new("data").join(res.to_string());
    let output_file =
        std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(output_path)?;

    cpio::write_cpio(entries.into_iter(), output_file)?;

    let _ = std::fs::OpenOptions::new().write(true).truncate(true).open(log);
    let _ = std::fs::remove_file(student_log);
    let _ = std::fs::remove_dir_all(result_dir);

    Ok(())
}

fn create_cpio_entry(
    path: impl AsRef<Path>,
) -> std::io::Result<(cpio::NewcBuilder, std::fs::File)> {
    let file_name = path
        .as_ref()
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or(std::io::Error::from(ErrorKind::InvalidFilename))?;

    let builder = cpio::NewcBuilder::new(file_name).uid(1000).gid(1000).mode(0o100_644);
    let file = std::fs::File::open(path)?;

    Ok((builder, file))
}

fn create_compressed_cpio_entry(
    path: impl AsRef<Path>,
) -> std::io::Result<(cpio::NewcBuilder, std::fs::File)> {
    let compressed_path = Path::new("/tmp").join(path.as_ref().file_name().unwrap());
    let compressed = std::fs::File::create(&compressed_path)?;

    zopfli::compress(
        Options::default(),
        zopfli::Format::Gzip,
        std::fs::File::open(path)?,
        &compressed,
    )?;

    create_cpio_entry(compressed_path)
}

/// List all files in a directory non-recursively
fn list_files(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let dir = std::fs::read_dir(path)?;
    let mut res = Vec::new();

    for e in dir {
        let e = e?;
        if e.file_type()?.is_file() {
            res.push(e.path());
        }
    }

    Ok(res)
}

fn directory_file_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let mut size = 0;
    for p in list_files(path)? {
        size += p.metadata()?.len();
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use crate::command::execute_program::{directory_file_size, list_files};
    use tempfile::tempdir;

    #[test]
    fn directory_files_and_size_are_correct() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        std::fs::create_dir(path.join("eyyy")).unwrap();
        std::fs::write(path.join("a"), [0; 10]).unwrap();
        std::fs::write(path.join("b"), [0; 15]).unwrap();

        let files = list_files(path).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&path.join("a")) && files.contains(&path.join("b")));

        assert_eq!(directory_file_size(path).unwrap(), 25);
    }
}
