use STS1_EDU_Scheduler::{command, communication};
use std::{io::{prelude::*, self}, ops::Deref, time::Duration};
use simplelog as sl;

mod common;
use common::prepare_program;

fn setup() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
}

#[test]
fn store_archive() {
    setup();
    let mut buf = Vec::new();
    std::fs::File::open("./tests/student_program.zip").unwrap().read_to_end(&mut buf).unwrap();

    command::store_archive("store".into(), buf).expect("store returns Err?");

    assert_eq!(0, std::process::Command::new("diff")
        .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/store"])
        .status().unwrap().code().unwrap());
    
    std::fs::remove_dir_all("./archives/store").unwrap();
}

#[test]
fn invalid_store() {
    setup();

    command::store_archive("dc".into(), vec![1, 2, 4, 5, 6]).expect_err("Should fail");
}

#[test]
fn execute_program_normal() {
    setup();
    prepare_program("normal");
    let mut ec: Option<command::ExecutionContext> = None;
    let ret = command::execute_program(&mut ec, "normal", "0", &Duration::from_secs(1)).expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}

    let mut res = String::new();    
    std::fs::File::open("./archives/normal/results/0")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");

    assert_eq!(res.replace("\r", ""), *"Some test results\nWith multiple lines\n".to_string());

    std::fs::remove_dir_all("./archives/normal").unwrap();
}

#[test]
fn execute_not_existing() {
    setup();
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "none", "existing", &Duration::from_secs(1)).expect_err("Should fail");
}

#[test]
fn execute_infinite_loop() {
    setup();
    prepare_program("inf");
    let mut ec: Option<command::ExecutionContext> = None;
    let ret = command::execute_program(&mut ec, "inf", "1", &Duration::from_secs(1)).expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}


    std::fs::remove_dir_all("./archives/inf").unwrap();
}

#[test]
fn execute_multiple() {
    setup();
    prepare_program("multiple");
    let mut ec: Option<command::ExecutionContext> = None;
    let ret = command::execute_program(&mut ec, "multiple", "1", &Duration::from_secs(1)).expect("execute returns Err?");
    let ret = command::execute_program(&mut ec, "multiple", "0", &Duration::from_secs(1)).expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}
    let ret = command::execute_program(&mut ec, "multiple", "0", &Duration::from_secs(1)).expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}

    // TODO assertions (check log?)

    std::fs::remove_dir_all("./archives/multiple").unwrap();
}

#[test]
fn stop_program() {
    setup();
    prepare_program("stop");
    let mut ec: Option<command::ExecutionContext> = None;
    let ret = command::execute_program(&mut ec, "stop", "2", &Duration::from_secs(1)).expect("execute returns Err?");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let ret = command::stop_program(&mut ec).expect("stop returns Err?");

    assert!(!ec.as_ref().unwrap().is_running(), "Program should be stopped");

    let mut res = String::new();
    std::fs::File::open("./archives/stop/results/2")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");
    assert_eq!(res.replace("\r", ""), *("First Line\n".to_string()));

    std::fs::remove_dir_all("./archives/stop");
}
