use STS1_EDU_Scheduler::{command, communication};
use std::{io::{prelude::*, self}, ops::Deref};
use simplelog as sl;


fn setup() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
}

fn prepare_program(path: &str) {
    let ret = std::fs::create_dir(format!("./archives/{}", path));
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
    let ret = std::fs::copy("./tests/test_data/main.py", format!("./archives/{}/main.py", path));
    if let Err(e) = ret {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Setup Error: {}", e);
        }
    }
}

#[test]
fn store_archive() {
    setup();
    let mut buf = Vec::new();
    std::fs::File::open("./tests/student_program.zip").unwrap().read_to_end(&mut buf).unwrap();

    command::store_archive("store", &buf).expect("store returns Err?");

    assert_eq!(0, std::process::Command::new("diff")
        .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/store"])
        .status().unwrap().code().unwrap());
    
    std::fs::remove_dir_all("./archives/store").unwrap();
}

#[test]
fn invalid_store() {
    setup();

    command::store_archive("dc", &vec![1, 2, 4, 5, 6]).expect_err("Should fail");
}

#[test]
fn execute_program_normal() {
    setup();
    prepare_program("normal");
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "normal", "0001").expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}

    let mut res = String::new();    
    std::fs::File::open("./archives/normal/results/0001/res.txt")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");

    assert_eq!(res, *"Some test results\nWith multiple lines\n".to_string());

    std::fs::remove_dir_all("./archives/normal").unwrap();
}

#[test]
fn execute_not_existing() {
    setup();
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "none", "existing").expect_err("Should fail");
}

#[test]
fn execute_infinite_loop() {
    setup();
    prepare_program("inf");
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "inf", "0002").expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}


    std::fs::remove_dir_all("./archives/inf").unwrap();
}

#[test]
fn execute_multiple() {
    setup();
    prepare_program("multiple");
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "multiple", "0002").expect("execute returns Err?");
    command::execute_program(&mut ec, "multiple", "0001").expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}
    command::execute_program(&mut ec, "multiple", "0001").expect("execute returns Err?");
    while ec.as_ref().unwrap().is_running() {}

    // TODO assertions (check log?)

    std::fs::remove_dir_all("./archives/multiple").unwrap();
}

#[test]
fn stop_program() {
    setup();
    prepare_program("stop");
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "stop", "0003").expect("execute returns Err?");
    std::thread::sleep(std::time::Duration::from_millis(500));
    command::stop_program(&mut ec).expect("stop returns Err?");

    assert!(!ec.as_ref().unwrap().is_running(), "Program should be stopped");

    let mut res = String::new();
    std::fs::File::open("./archives/stop/results/0003/res.txt")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");
    assert_eq!(res, *("First Line\n".to_string()));

    std::fs::remove_dir_all("./archives/stop").unwrap();
}

struct MockCom {
    pub last_path: std::path::PathBuf
}
impl MockCom {
    fn new() -> MockCom {
        MockCom {last_path: std::path::PathBuf::new()}
    }
}

impl communication::Communication for MockCom {
    fn send(&mut self, p: std::path::PathBuf) {
        self.last_path = p;
    }

    fn receive(&self) -> std::path::PathBuf {
        unimplemented!();
    }
}

#[test]
fn return_results_normal() {
    setup();
    prepare_program("res");
    let mut ec: Option<command::ExecutionContext> = None;
    command::execute_program(&mut ec, "res", "0001").unwrap();
    while ec.as_ref().unwrap().is_running() {}
    let mut com = MockCom::new();
    command::return_results(&mut com, "res", "0001").expect("results returns Err?");    

    assert_eq!(com.last_path, std::path::PathBuf::from("./data/res0001.zip"));
    assert!(com.last_path.metadata().unwrap().len() > 700, "Output should be larger");
    assert!(std::path::Path::new("log").metadata().unwrap().len() == 0, "Log is not cleared");
    assert!(!std::path::Path::new("./archives/res/results/0001").exists(), "Results are not deleted");

    std::fs::remove_file("./data/res0001.zip").unwrap();
    std::fs::remove_dir_all("./archives/res").unwrap();
}

#[test]
fn return_results_none() {
    setup();
    let mut com = MockCom::new();
    command::return_results(&mut com, "none", "existing").expect("results returns Err?");

    assert_eq!(com.last_path, std::path::PathBuf::from("./data/noneexisting.zip"));
    assert!(com.last_path.metadata().unwrap().len() > 4, "Should contain log");
    assert!(com.last_path.exists());
    std::fs::remove_file(com.last_path).unwrap();
}