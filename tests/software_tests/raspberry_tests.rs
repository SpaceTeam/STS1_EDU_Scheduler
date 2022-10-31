use STS1_EDU_Scheduler::communication::{CSBIPacket, CSBIPacket::*};

/// These tests are for use with the actual raspberry hardware
///
/// Use `cargo test --features rpi` to run them

#[cfg(feature = "rpi")]
#[test]
fn build_pack_store_archive() {
    
    let packets = vec![
        // Create a list of packets to send
        DATA(vec![1, 0, 0]), // Store Archive
        DATA(std::fs::read("tests/student_program.zip").unwrap()), // bytes of the zip
        EOF,                 // End of File
        
    ];

    std::fs::write("tests/store_archive.pack", combine_packets(packets)); // Save the packets into a .pack file
}

#[cfg(feature = "rpi")]
#[test]
fn build_pack_execute_program() {
    // Execute with Program ID 0 & Queue ID 0
    std::fs::write("tests/execute_program.pack", DATA(vec![2, 0, 0, 0, 0, 5, 0]).serialize());
}

#[cfg(feature = "rpi")]
#[test]
fn build_pack_get_status() {
    std::fs::write("tests/get_status.pack", DATA(vec![4]).serialize());
}

#[cfg(feature = "rpi")]
#[test]
fn build_pack_return_result() {
    std::fs::write("tests/return_result.pack", DATA(vec![5]).serialize());
}

fn combine_packets(list: Vec<CSBIPacket>) -> Vec<u8> {
    let mut v = Vec::new();
    for p in list {
        v.extend(p.serialize());
    }
    v
}
