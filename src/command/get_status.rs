use crate::communication::{CEPPacket, CommunicationHandle};

use super::{check_length, CommandResult, Event, SyncExecutionContext};

/// The function handles the get status command, by checking if either a status or result is enqueued.
/// A status always has priority over a result.
pub fn get_status(
    data: Vec<u8>,
    com: &mut impl CommunicationHandle,
    exec: &mut SyncExecutionContext,
) -> CommandResult {
    check_length(com, &data, 1)?;

    let mut l_exec = exec.lock().unwrap();
    if !l_exec.has_data_ready() {
        com.send_packet(&CEPPacket::Data(vec![0]))?;
        return Ok(());
    }

    if let Some(index) =
        l_exec.event_vec.as_ref().iter().position(|x| matches!(x, Event::Status(_)))
    {
        let event = l_exec.event_vec[index];
        com.send_packet(&CEPPacket::Data(event.to_bytes()))?;
        l_exec.event_vec.remove(index)?;
    } else {
        let event = *l_exec.event_vec.as_ref().last().unwrap(); // Safe, because we know it is not empty
        com.send_packet(&CEPPacket::Data(event.to_bytes()))?;

        if !matches!(event, Event::Result(_)) {
            // Results are removed when deleted
            l_exec.event_vec.pop()?;
        }
    }

    l_exec.check_update_pin();
    Ok(())
}
