use crate::simulation::*;

#[test]
fn simulate_archive_is_stored_correctly() -> Result<(), std::io::Error> {
    let (_sched, mut com, _socat) = start_scheduler("archive_is_stored_correctly").unwrap();

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
    let (_sched, mut com, _socat) = start_scheduler("return_result_retries").unwrap();

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

#[test]
fn result_is_deleted_after_transfer() {
    let (_sched, mut com, _socat) = start_scheduler("results_deleted").unwrap();

    simulate_test_store_archive(&mut com, 8).unwrap();
    simulate_execute_program(&mut com, 8, 3, 5).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    assert_eq!(simulate_get_status(&mut com).unwrap(), get_status_program_finished(8, 3, 0));
    assert_eq!(simulate_get_status(&mut com).unwrap(), get_status_result_ready(8, 3));

    simulate_return_result(&mut com, 8, 3).unwrap();
    com.send_packet(&CEPPacket::Ack).unwrap();

    assert_eq!(simulate_get_status(&mut com).unwrap(), vec![0]);
    assert_eq!(std::fs::read_dir("tests/tmp/results_deleted/data").unwrap().count(), 0);
    assert_eq!(
        std::fs::read_dir("tests/tmp/results_deleted/archives/8/results").unwrap().count(),
        0
    );
}
