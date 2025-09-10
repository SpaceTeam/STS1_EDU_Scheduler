use std::path::Path;

use crate::simulation::*;

#[test]
fn logfile_is_created() {
    let (_sched, _com, _socat) = start_scheduler("log_created").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(400));

    assert!(std::path::Path::new("./tests/tmp/log_created/log").exists());
}

#[test]
fn logfile_is_cleared_after_sent() -> std::io::Result<()> {
    let (_sched, mut com, _socat) = start_scheduler("log_is_cleared_after_sent").unwrap();

    simulate_test_store_archive(&mut com, 1).unwrap();
    com.send_packet(&CEPPacket::Data(execute_program(1, 0, 3))).unwrap();
    com.await_ack(Duration::MAX).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let _ = simulate_return_result(&mut com, 1, 0).unwrap();
    com.send_packet(&CEPPacket::Ack).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    assert!(
        file_size("./tests/tmp/log_is_cleared_after_sent/log")? < 100,
        "Logfile is not cleared"
    );

    Ok(())
}

#[test]
fn permalog_is_truncated() -> std::io::Result<()> {
    // Fill up log well above 1 MB
    {
        let (_sched, mut com, _socat) = start_scheduler("permalog_is_truncated")?;
        for _ in 0..50_000 {
            simulate_get_status(&mut com).unwrap();
        }
        assert!(file_size("./tests/tmp/permalog_is_truncated/permalog")? > 1_000_000);
    }
    // Restart
    let _sched = spawn_scheduler(&Path::new("tests/tmp").join("permalog_is_truncated"))?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(file_size("./tests/tmp/permalog_is_truncated/permalog")? < 100);

    Ok(())
}

fn file_size(file: impl AsRef<Path>) -> std::io::Result<u64> {
    Ok(std::fs::File::open(file)?.metadata()?.len())
}
