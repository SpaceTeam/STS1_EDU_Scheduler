use STS1_EDU_Scheduler::communication;
use communication::CSBIPacket;

#[test]
fn csbi_command() {
    assert!(CSBIPacket::ACK.serialize() == vec![0xd7u8]);
}

#[test]
fn csbi_data() {
    let b = vec![0x12u8, 0x34, 0x56];
    assert_eq!(CSBIPacket::DATA(b).serialize(), vec![0x8bu8, 0x00, 0x03, 0x12, 0x34, 0x56, 0xbe, 0x98, 0x86, 0x57]);
}