use super::*;

#[test]
fn result_files_too_large_are_not_packed() {
    let (_sched, mut com, _socat) =
        start_scheduler("result_files_too_large_are_not_packed").unwrap();

    simulate_test_store_archive(&mut com, 1).unwrap();
    simulate_execute_program(&mut com, 1, 5, 3).unwrap();
    std::thread::sleep(Duration::from_secs(1));

    let result = simulate_return_result(&mut com, 1, 5).unwrap();
    let mut result = simple_archive::Reader::new(&result[..]).map(Result::unwrap);

    assert!(!result.any(|e| e.path == "1_5"));
}
