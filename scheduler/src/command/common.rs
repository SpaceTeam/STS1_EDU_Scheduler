use super::{CommandError, CommandResult, SyncExecutionContext};
use crate::communication::{CEPPacket, CommunicationHandle};
use anyhow::anyhow;
use std::time::Duration;

pub fn check_length(
    com: &mut impl CommunicationHandle,
    vec: &[u8],
    n: usize,
) -> Result<(), CommandError> {
    let actual_len = vec.len();
    if actual_len == n {
        Ok(())
    } else {
        com.send_packet(&CEPPacket::Nack)?;
        Err(CommandError::ProtocolViolation(anyhow!(
            "Received command with {actual_len} bytes, expected {n}"
        )))
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
                .or(Err(CommandError::NonRecoverable(anyhow!("Supervisor thread panicked"))))?;
            return Ok(());
        }
    }

    Err(CommandError::NonRecoverable(anyhow!("Supervisor thread did not finish in time")))
}
