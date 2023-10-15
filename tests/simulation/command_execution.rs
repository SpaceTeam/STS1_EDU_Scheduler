use crate::simulation::*;

#[test]
fn simulate_archive_is_stored_correctly() -> Result<(), std::io::Error> {
    let (mut scheduler, mut serial_port) = start_scheduler("archive_is_stored_correctly")?;
    let mut cobc_in = serial_port.stdout.take().unwrap();
    let mut cobc_out = serial_port.stdin.take().unwrap();

    simulate_test_store_archive(&mut cobc_in, &mut cobc_out)?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    assert_eq!(
        0,
        std::process::Command::new("diff") // check wether the archive was stored correctly
            .args(["-yq", "--strip-trailing-cr", "tests/test_data", "tests/tmp/archive_is_stored_correctly/archives/1"])
            .status()?
            .code()
            .unwrap()
    );

    scheduler.kill()?;
    Ok(())
}