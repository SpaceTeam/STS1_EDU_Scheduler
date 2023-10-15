use crate::simulation::*;

#[test]
fn logfile_is_created() -> Result<(), std::io::Error> {
    let (mut scheduler, _) = start_scheduler("log_created")?;
    std::thread::sleep(std::time::Duration::from_millis(400));
    scheduler.kill().unwrap();

    assert!(std::path::Path::new("./tests/tmp/log_created/log").exists());
    Ok(())
}