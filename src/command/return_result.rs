use super::{truncate_to_size, CommandResult, SyncExecutionContext};
use crate::{
    command::{check_length, CommandError, Event, ResultId, COMMAND_TIMEOUT},
    communication::{CEPPacket, CommunicationHandle},
};

/// Handles a complete return result command. The result tar file is only deleted if a final Ack is
/// received.
pub fn return_result(
    data: &[u8],
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, data, 7)?;

    let program_id = u16::from_le_bytes([data[1], data[2]]);
    let timestamp = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let result_path = format!("./data/{program_id}_{timestamp}.tar");

    if !std::path::Path::new(&result_path).exists() {
        com.send_packet(&CEPPacket::Nack)?;
        return Err(CommandError::ProtocolViolation(
            format!("Result {program_id}:{timestamp} does not exist").into(),
        ));
    }

    let bytes = std::fs::read(result_path)?;
    log::info!("Returning result for {}:{}", program_id, timestamp);
    com.send_multi_packet(&bytes)?;

    com.await_ack(COMMAND_TIMEOUT)?;
    let result_id = ResultId { program_id, timestamp };
    delete_result(result_id)?;

    let mut l_exec = exec.lock().unwrap();
    if let Some(event_index) =
        l_exec.event_vec.as_ref().iter().position(|x| x.event == Event::Result(result_id))
    {
        l_exec.event_vec.remove(event_index)?;
    } else {
        log::error!("Could not find event entry for existing result file {program_id}:{timestamp}");
    }

    l_exec.configure_update_pin();
    Ok(())
}

/// Deletes the result archive corresponding to the next element in the result queue and removes
/// that element from the queue. The update pin is updated accordingly
fn delete_result(res: ResultId) -> CommandResult {
    let res_path = format!("./archives/{}/results/{}", res.program_id, res.timestamp);
    let log_path = format!("./data/{}_{}.log", res.program_id, res.timestamp);
    let out_path = format!("./data/{}_{}.tar", res.program_id, res.timestamp);
    let _ = std::fs::remove_file(res_path);
    let _ = std::fs::remove_file(log_path);
    let _ = std::fs::remove_file(out_path);

    if let Ok(mut file) = std::fs::File::options().write(true).open("log") {
        truncate_to_size(&mut file, 0)?;
    }

    Ok(())
}
