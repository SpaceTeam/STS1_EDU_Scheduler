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

#[test]
fn return_result_is_retried_n_times() {
    let (mut com, _socat) = SimulationComHandle::with_socat_proc("return_result_retries");
    let _sched = start_scheduler("return_result_retries").unwrap();

    simulate_test_store_archive(&mut com, 8).unwrap();
    simulate_execute_program(&mut com, 8, 0, 5).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));

    assert_eq!(get_status_program_finished(8, 0, 0), simulate_get_status(&mut com).unwrap());
    for i in 0..5 {
        assert_eq!(get_status_result_ready(8, 0), simulate_get_status(&mut com).unwrap());
        dbg!(i);
    }
    assert_eq!([0u8], *simulate_get_status(&mut com).unwrap());
}
