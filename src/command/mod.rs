mod common;
mod error;
mod execute_program;
mod execution_context;
mod get_status;
mod return_result;
mod stop_program;
mod store_archive;
mod update_time;

use crate::communication::{CEPPacket, CommunicationHandle};
use anyhow::anyhow;
pub use common::*;
pub use error::CommandError;
use execute_program::execute_program;
pub use execution_context::*;
use get_status::get_status;
use return_result::return_result;
use std::time::Duration;
use stop_program::stop_program;
use store_archive::store_archive;
use update_time::update_time;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(1);

type CommandResult = Result<(), CommandError>;

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn handle_command(com: &mut impl CommunicationHandle, exec: &mut SyncExecutionContext) {
    let ret = process_command(com, exec);

    match ret {
        Ok(()) => log::info!("Command executed successfully"),

        Err(CommandError::NonRecoverable(e)) => {
            log::error!("Non-Recoverable error: {e}");
            panic!("Aborting now {e:?}");
        }
        Err(CommandError::ProtocolViolation(e)) => {
            log::error!("Protocol Violation: {e}");
        }
        Err(CommandError::External(e)) => {
            log::error!("External error: {e}");
        }
    };
}

pub fn process_command(
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    let packet = com.receive_packet()?;
    let CEPPacket::Data(data) = packet else {
        return Err(CommandError::NonRecoverable(anyhow!(
            "Received {packet:?} as command start, expected Data"
        )));
    };

    if data.is_empty() {
        return Err(CommandError::ProtocolViolation(anyhow!("Received empty data packet")));
    }

    match data.first().unwrap() {
        0x01 => store_archive(&data, com, exec)?,
        0x02 => execute_program(&data, com, exec)?,
        0x03 => stop_program(&data, com, exec)?,
        0x04 => get_status(&data, com, exec)?,
        0x05 => return_result(&data, com, exec)?,
        0x06 => update_time(&data, com, exec)?,
        b => {
            return Err(CommandError::ProtocolViolation(anyhow!("Unknown command {b:#x}")));
        }
    };

    Ok(())
}
