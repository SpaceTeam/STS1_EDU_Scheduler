use std::time::Duration;
use STS1_EDU_Scheduler::communication::{CommunicationHandle, CEPPacket};
use super::{SimulationComHandle, start_scheduler, get_status};

#[test]
fn integrity_ack_timeout_is_honored() {
    let (mut cobc, _socat) = SimulationComHandle::with_socat_proc("integrity_timeout");
    let _sched = start_scheduler("integrity_timeout").unwrap();

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
