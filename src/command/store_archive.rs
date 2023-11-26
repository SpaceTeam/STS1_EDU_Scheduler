use std::{io::Write, process::Command};

use super::{CommandError, CommandResult, SyncExecutionContext};
use crate::{
    command::{check_length, COM_TIMEOUT_DURATION},
    communication::{CEPPacket, CommunicationHandle},
};

/// This function implements the Store Archive command, including the reception of the archive itself
pub fn store_archive(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    _exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 3)?;
    com.send_packet(CEPPacket::ACK)?;

    let id = u16::from_le_bytes([data[1], data[2]]).to_string();
    log::info!("Storing Archive {}", id);

    let bytes = com.receive_multi_packet(&COM_TIMEOUT_DURATION, || false)?; // !! TODO !!
    unpack_archive(id, bytes)?;

    com.send_packet(CEPPacket::ACK)?;
    Ok(())
}

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
fn unpack_archive(folder: String, bytes: Vec<u8>) -> CommandResult {
    // Store bytes into temporary file
    // Store bytes into temporary file
    let zip_path = format!("./data/{}.zip", folder);
    let mut zip_file = std::fs::File::create(&zip_path)?;
    zip_file.write_all(&bytes)?;
    zip_file.sync_all()?;

    let exit_status = Command::new("unzip")
        .arg("-o") // overwrite silently
        .arg(&zip_path)
        .arg("-d") // target directory
        .arg(format!("./archives/{}", folder))
        .status();

    // Remove the temporary file, even if unzip failed
    std::fs::remove_file(zip_path)?;

    match exit_status {
        Ok(status) => {
            if !status.success() {
                return Err(CommandError::NonRecoverable("unzip failed".into()));
            }
        }
        Err(err) => {
            return Err(err.into());
        }
    }

    Ok(())
}
