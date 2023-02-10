use crate::communication::{CSBIPacket, CommunicationError, CommunicationHandle};
use std::time::Duration;

mod handlers;
pub use handlers::*;
mod execution_context;
pub use execution_context::*;
mod error;
pub use error::CommandError;

type CommandResult = Result<(), CommandError>;

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn handle_command(
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    let ret = process_command(com, exec);

    ret
}

pub fn process_command(
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    let packet = com.receive_packet(&Duration::MAX)?;
    let data = match packet {
        CSBIPacket::DATA(data) => data,
        _ => {
            return Err(CommandError::ProtocolViolation(
                format!("Received {:?} as command start, expected DATA", packet).into(),
            ));
        }
    };

    if data.is_empty() {
        return Err(CommandError::ProtocolViolation("No data sent with data packet".into()));
    }

    match data[0] {
        0x01 => store_archive(data, com, exec)?,
        0x02 => execute_program(data, com, exec)?,
        0x03 => stop_program(data, com, exec)?,
        0x04 => get_status(data, com, exec)?,
        0x05 => return_result(data, com, exec)?,
        0x06 => update_time(data, com, exec)?,
        b => {
            return Err(CommandError::ProtocolViolation(
                format!("Unknown command {:#x}", b).into(),
            ));
        }
    };

    Ok(())
}
