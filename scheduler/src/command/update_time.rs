use super::{check_length, CommandError, CommandResult, SyncExecutionContext};
use crate::communication::{CEPPacket, CommunicationHandle};
use anyhow::anyhow;
use std::process::Command;

/// Handles the update time command
pub fn update_time(
    data: &[u8],
    com: &mut impl CommunicationHandle,
    _exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, data, 5)?;

    let time = i32::from_le_bytes([data[1], data[2], data[3], data[4]]);
    set_system_time(time)?;

    com.send_packet(&CEPPacket::Ack)?;
    Ok(())
}

fn set_system_time(s_since_epoch: i32) -> CommandResult {
    let exit_status = Command::new("date").arg("-s").arg(format!("@{s_since_epoch}")).status()?;
    if !exit_status.success() {
        return Err(CommandError::NonRecoverable(anyhow!("date utility failed")));
    }

    Ok(())
}
