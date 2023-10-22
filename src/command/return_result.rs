use crate::{
    command::{check_length, CommandError, Event, ResultId, COM_TIMEOUT_DURATION},
    communication::{CEPPacket, CommunicationHandle},
};

use super::{truncate_to_size, CommandResult, SyncExecutionContext};

/// Handles a complete return result command. The result zip file is only deleted if a final ACK is
/// received.
pub fn return_result(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(&data, 7)?;
    com.send_packet(CEPPacket::ACK)?;

    let program_id = u16::from_le_bytes([data[1], data[2]]);
    let timestamp = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let result_path = format!("./data/{}_{}.zip", program_id, timestamp);

    if !std::path::Path::new(&result_path).exists() {
        return Err(CommandError::ProtocolViolation(
            format!("Result {}:{} does not exist", program_id, timestamp).into(),
        ));
    }

    let bytes = std::fs::read(result_path)?;
    log::info!("Returning result for {}:{}", program_id, timestamp);
    com.send_multi_packet(bytes, &COM_TIMEOUT_DURATION)?;

    let response = com.receive_packet(&COM_TIMEOUT_DURATION)?;
    if response == CEPPacket::ACK {
        let result_id = ResultId { program_id, timestamp };
        delete_result(result_id)?;

        let mut l_exec = exec.lock().unwrap();
        let event_index =
            l_exec.event_vec.as_ref().iter().position(|x| x == &Event::Result(result_id)).unwrap();
        l_exec.event_vec.remove(event_index)?;
        l_exec.check_update_pin();
        drop(l_exec);
    } else {
        log::error!("COBC did not acknowledge result");
    }

    Ok(())
}

/// Deletes the result archive corresponding to the next element in the result queue and removes
/// that element from the queue. The update pin is updated accordingly
fn delete_result(res: ResultId) -> CommandResult {
    let res_path = format!("./archives/{}/results/{}", res.program_id, res.timestamp);
    let log_path = format!("./data/{}_{}.log", res.program_id, res.timestamp);
    let out_path = format!("./data/{}_{}.zip", res.program_id, res.timestamp);
    let _ = std::fs::remove_file(res_path);
    let _ = std::fs::remove_file(log_path);
    let _ = std::fs::remove_file(out_path);
    let _ = truncate_to_size("log", 0);

    Ok(())
}