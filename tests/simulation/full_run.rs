use crate::simulation::*;
use std::{io::Cursor, time::Duration};

#[test]
fn full_run() {
    let (_sched, mut com, _socat) = start_scheduler("full_run").unwrap();

    // store and execute program
    simulate_test_store_archive(&mut com, 1).unwrap();
    simulate_execute_program(&mut com, 1, 3, 3).unwrap();
    std::thread::sleep(Duration::from_secs(1));

    // read program finished and result ready
    assert_eq!(simulate_get_status(&mut com).unwrap(), [1, 1, 0, 3, 0, 0, 0, 0]);
    assert_eq!(simulate_get_status(&mut com).unwrap(), [2, 1, 0, 3, 0, 0, 0]);

    // Check result
    let result = simulate_return_result(&mut com, 1, 3).unwrap();
    let mut result_archive = tar::Archive::new(Cursor::new(result));
    com.send_packet(&CEPPacket::Ack).unwrap();

    let result_file = result_archive
        .entries()
        .unwrap()
        .find(|x| x.as_ref().unwrap().header().path().unwrap().ends_with("3"))
        .unwrap()
        .unwrap();
    assert_eq!(result_file.bytes().map(|b| b.unwrap()).collect::<Vec<_>>(), vec![0xde, 0xad]);

    assert_eq!(simulate_get_status(&mut com).unwrap(), [0]);
}
