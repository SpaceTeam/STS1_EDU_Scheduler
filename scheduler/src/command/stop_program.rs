use super::{check_length, terminate_student_program, CommandResult, SyncExecutionContext};
use crate::communication::{CEPPacket, CommunicationHandle};

/// Stops the currently running student program
pub fn stop_program(
    data: &[u8],
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, data, 1)?;

    terminate_student_program(exec).expect("to terminate student program");

    com.send_packet(&CEPPacket::Ack)?;
    Ok(())
}
