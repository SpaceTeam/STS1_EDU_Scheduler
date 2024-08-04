use super::{get_status, start_scheduler};
use std::time::Duration;
use STS1_EDU_Scheduler::communication::{CEPPacket, CommunicationHandle};

#[test]
fn integrity_ack_timeout_is_honored() {
    let (_sched, mut cobc, _socat) = start_scheduler("integrity_timeout").unwrap();

    // Check that delayed ACK is allowed
    cobc.send_packet(&CEPPacket::Data(get_status())).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(cobc.receive_packet().unwrap(), CEPPacket::Data(vec![0]));

    cobc.send_packet(&CEPPacket::Data(get_status())).unwrap();
    assert_eq!(CEPPacket::try_from_read(&mut cobc.cobc_in).unwrap(), CEPPacket::Data(vec![0])); // No ACK sent!
    std::thread::sleep(Duration::from_millis(1010));

    // Timeout passed, normal communication should be possible
    cobc.send_packet(&CEPPacket::Data(get_status())).unwrap();
    assert_eq!(cobc.receive_packet().unwrap(), CEPPacket::Data(vec![0]));
}
