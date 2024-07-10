use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use super::{simulate_get_status, start_scheduler};

#[test]
fn dosimeter_events_are_added() {
    let (_sched, mut com, _socat) = start_scheduler("dosimeter").unwrap();

    std::thread::sleep(Duration::from_millis(200));

    {
        let mut socket = UnixStream::connect("/tmp/STS1_EDU_Scheduler_SIM_dosimeter").unwrap();
        writeln!(socket, "dosimeter/on").unwrap();
    }

    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(simulate_get_status(&mut com).unwrap(), [0x03]);
}

#[test]
fn multiple_dosimeter_events() {
    let (_sched, mut com, _socat) = start_scheduler("dosimeter_multi").unwrap();

    std::thread::sleep(Duration::from_millis(200));

    let mut socket = UnixStream::connect("/tmp/STS1_EDU_Scheduler_SIM_dosimeter_multi").unwrap();
    for _ in 0..10 {
        writeln!(socket, "dosimeter/on").unwrap();
        writeln!(socket, "dosimeter/off").unwrap();
    }

    std::thread::sleep(Duration::from_millis(200));
    for _ in 0..10 {
        assert_eq!(simulate_get_status(&mut com).unwrap(), [0x03]);
        assert_eq!(simulate_get_status(&mut com).unwrap(), [0x04]);
    }
}
