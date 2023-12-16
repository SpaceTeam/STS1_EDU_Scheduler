use super::{CommandError, CommandResult, SyncExecutionContext};
use crate::communication::{CEPPacket, CommunicationHandle};
use std::time::Duration;

pub fn check_length(
    com: &mut impl CommunicationHandle,
    vec: &Vec<u8>,
    n: usize,
) -> Result<(), CommandError> {
    let actual_len = vec.len();
    if actual_len != n {
        log::error!("Command came with {actual_len} bytes, should have {n}");
        com.send_packet(&CEPPacket::Nack)?;
        Err(CommandError::ProtocolViolation(
            format!("Received command with {actual_len} bytes, expected {n}").into(),
        ))
    } else {
        Ok(())
    }
}

/// Truncates the file at `path` to the given size. Returns wether it actually had to truncate.
pub fn truncate_to_size(path: &str, n_bytes: u64) -> Result<bool, std::io::Error> {
    log::info!("Truncating {:?}", &path);
    let file = std::fs::File::options().write(true).open(path)?;
    let size = file.metadata()?.len();
    if size > n_bytes {
        file.set_len(n_bytes)?;
        file.sync_all()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// If no program is currently running, this function simply returns. Otherwise it signals the
/// supervisor thread to kill the student program and waits for a maximum of 2s before returning
/// and error
pub fn terminate_student_program(exec: &mut SyncExecutionContext) -> CommandResult {
    let mut con = exec.lock().unwrap();
    if !con.is_student_program_running() {
        return Ok(());
    }
    con.running_flag = false; // Signal watchdog thread to terminate
    drop(con); // Release mutex

    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100)); // Sensible amount?
        let mut con = exec.lock().unwrap();
        if con.thread_handle.as_ref().unwrap().is_finished() {
            con.thread_handle
                .take()
                .unwrap()
                .join()
                .or(Err(CommandError::NonRecoverable("Supervisor thread panicked".into())))?;
            return Ok(());
        }
    }

    Err(CommandError::NonRecoverable("Supervisor thread did not finish in time".into()))
}
