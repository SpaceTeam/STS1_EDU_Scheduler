use crate::communication::{CSBIPacket, CommunicationError, CommunicationHandle};
use std::time::Duration;

mod handlers;
pub use handlers::*;
mod execution_context;
pub use execution_context::*;
mod error;
pub use error::CommandError;

type CommandResult = Result<(), CommandError>;

const COM_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::new(2, 0);

/// Main routine. Waits for a command to be received from the COBC, then parses and executes it.
pub fn handle_command(
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    let ret = process_command(com, exec);

    if let Err(ce) = &ret {
        match ce {
            e @ CommandError::SystemError(_)
            | e @ CommandError::InvalidCommError
            | e @ CommandError::CommunicationError(CommunicationError::CRCError) => {
                log::error!("Failed to process command {:?}", e);
                com.send_packet(CSBIPacket::NACK)?;
            }
            _ => {}
        }
    }

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
            log::error!("Received {:?} as command start, expected DATA", packet);
            return Err(CommandError::CommunicationError(CommunicationError::PacketInvalidError));
        }
    };

    if data.is_empty() {
        log::error!("No data received");
        return Err(CommandError::InvalidCommError);
    }

    match data[0] {
        0x01 => {
            store_archive(data, com, exec)?;
        }
        0x02 => {
            execute_program(data, com, exec)?;
        }
        0x03 => {
            stop_program(data, com, exec)?;
        }
        0x04 => {
            get_status(data, com, exec)?;
        }
        0x05 => {
            // RETURN RESULT
            check_length(&data, 1)?;
            com.send_packet(CSBIPacket::ACK)?;
            log::info!("Returning Result");
            com.send_multi_packet(return_result(exec)?, &COM_TIMEOUT_DURATION)?;
            if let CSBIPacket::ACK = com.receive_packet(&COM_TIMEOUT_DURATION)? {
                delete_result(exec)?;
            } else {
                log::error!("COBC did not acknowledge result");
            }
        }
        0x06 => {
            // UPDATE TIME
            check_length(&data, 5)?;
            com.send_packet(CSBIPacket::ACK)?;
            let time = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);
            log::info!("Updating Time to {}", time);
            update_time(time)?;
            com.send_packet(CSBIPacket::ACK)?;
        }
        b => {
            log::error!("Received command {}", b);
            return Err(CommandError::InvalidCommError);
        }
    };

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
