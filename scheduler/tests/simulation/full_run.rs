use crate::simulation::*;
use std::time::Duration;

#[test]
fn full_run() -> anyhow::Result<()> {
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
    com.send_packet(&CEPPacket::Ack).unwrap();

    let mut results = cpio::NewcReader::new(std::io::Cursor::new(result))?;
    while !results.entry().is_trailer() {
        match results.entry().name() {
            "result" => {
                let mut buf = [0; 2];
                results.read_exact(&mut buf)?;
                assert_eq!(buf, [0xde, 0xad]);
            }
            "log" | "1_3.log" => (),
            n => panic!("Invalid name {n}"),
        }

        results = cpio::NewcReader::new(results.skip()?)?;
    }

    assert_eq!(simulate_get_status(&mut com).unwrap(), [0]);

    Ok(())
}
