use crate::simulation::*;

#[test]
fn logfile_is_created() -> Result<(), std::io::Error> {
    let (mut scheduler, _) = start_scheduler("log_created")?;
    std::thread::sleep(std::time::Duration::from_millis(400));
    scheduler.kill().unwrap();

    assert!(std::path::Path::new("./tests/tmp/log_created/log").exists());
    Ok(())
}

#[test]
fn logfile_is_cleared_after_sent() -> std::io::Result<()> {
    let (mut scheduler, mut serial_port) = start_scheduler("log_is_cleared_after_sent")?;
    let mut cobc_in = serial_port.stdout.take().unwrap();
    let mut cobc_out = serial_port.stdin.take().unwrap();

    simulate_test_store_archive(&mut cobc_in, &mut cobc_out, 1).unwrap();
    simulate_execute_program(&mut cobc_in, &mut cobc_out, 1, 0, 3).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = simulate_return_result(&mut cobc_in, &mut cobc_out, 1, 0).unwrap();
    cobc_out.write_all(&CEPPacket::ACK.serialize()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    scheduler.kill().unwrap();

    let log_metadata = std::fs::metadata("./tests/tmp/log_is_cleared_after_sent/log")?;
    assert!(log_metadata.len() < 50, "Logfile is not empty");

    Ok(())
}
