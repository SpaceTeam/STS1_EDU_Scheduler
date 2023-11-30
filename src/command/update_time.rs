use std::process::Command;

use crate::communication::{CEPPacket, CommunicationHandle};

use super::{check_length, CommandError, CommandResult, SyncExecutionContext};

/// Handles the update time command
pub fn update_time(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    _exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, &data, 5)?;

    let time = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);
    set_system_time(time)?;

    com.send_packet(&CEPPacket::ACK)?;
    Ok(())
}

fn set_system_time(s_since_epoch: i32) -> CommandResult {
    let exit_status = Command::new("date").arg("-s").arg(format!("@{}", s_since_epoch)).status()?;
    if !exit_status.success() {
        return Err(CommandError::NonRecoverable("date utility failed".into()));
    }

    Ok(())
}
