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

    let result = {
        let mut events = l_exec.event_vec.as_mut();
        let index = events.iter().position(|x| matches!(x.event, Event::Status(_))).unwrap_or(0);

        events[index].retries -= 1;
        let result = com.send_packet(&CEPPacket::Data(events[index].event.into()));
        if !matches!(events[index].event, Event::Result(_)) || events[index].retries == 0 {
            events.remove(index);
        }
        result
    };

    l_exec.configure_update_pin();
    Ok(result?)
}
