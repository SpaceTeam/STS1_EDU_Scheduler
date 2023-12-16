use crate::simulation::*;

#[test]
fn simulate_archive_is_stored_correctly() -> Result<(), std::io::Error> {
    let (mut com, _socat) = SimulationComHandle::with_socat_proc("archive_is_stored_correctly");
    let _sched = start_scheduler("archive_is_stored_correctly")?;

    simulate_test_store_archive(&mut com, 1).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));

    assert_eq!(
        0,
        std::process::Command::new("diff") // check wether the archive was stored correctly
            .args([
                "-yq",
                "--strip-trailing-cr",
                "tests/test_data",
                "tests/tmp/archive_is_stored_correctly/archives/1"
            ])
            .status()?
            .code()
            .unwrap()
    );

    Ok(())
}
