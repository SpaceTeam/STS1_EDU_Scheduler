use crate::simulation::*;

#[test]
fn logfile_is_created() -> Result<(), std::io::Error> {
    let (_, _proc) = SimulationComHandle::with_socat_proc("log_created");
    let _sched = start_scheduler("log_created")?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    assert!(std::path::Path::new("./tests/tmp/log_created/log").exists());
    Ok(())
}

#[test]
fn logfile_is_cleared_after_sent() -> std::io::Result<()> {
    let (mut com, _socat) = SimulationComHandle::with_socat_proc("log_is_cleared_after_sent");
    let _sched = start_scheduler("log_is_cleared_after_sent")?;

    simulate_test_store_archive(&mut com, 1).unwrap();
    com.send_packet(&CEPPacket::Data(execute_program(1, 0, 3))).unwrap();
    com.await_ack(Duration::MAX).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let _ = simulate_return_result(&mut com, 1, 0).unwrap();
    com.send_packet(&CEPPacket::Ack).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let log_metadata = std::fs::metadata("./tests/tmp/log_is_cleared_after_sent/log")?;
    assert!(log_metadata.len() < 50, "Logfile is not empty");

    Ok(())
}
