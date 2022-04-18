use STS1_EDU_Scheduler::command::*;
use std::io::prelude::*;
use simplelog as sl;


fn setup() {
    let _ = sl::SimpleLogger::init(sl::LevelFilter::Error, sl::Config::default());
}

fn prepare_program(path: &str) {
    std::fs::create_dir(format!("./archives/{}", path)).unwrap();
    std::fs::copy("./tests/test_data/main.py", format!("./archives/{}/main.py", path)).unwrap();
}

#[test]
fn store_archive() {
    setup();
    let mut buf = Vec::new();
    std::fs::File::open("./tests/student_program.zip").unwrap().read_to_end(&mut buf).unwrap();
    CommandHandler::store_archive("testa", buf);

    assert_eq!(0, std::process::Command::new("diff")
        .args(["-yq", "--strip-trailing-cr", "tests/test_data", "archives/testa"])
        .status().unwrap().code().unwrap());
    
    std::fs::remove_dir_all("./archives/testa").unwrap();
}

#[test]
fn execute_program_normal() {
    setup();
    prepare_program("normal");
    let mut ch = CommandHandler::create();
    ch.execute_program("normal", "0001");
    while ch.is_program_running() {}
    let mut res = String::new();
    std::fs::File::open("./archives/normal/results/0001/res.txt")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");

    assert_eq!(res, *"Some test results\nWith multiple lines\n".to_string());

    std::fs::remove_dir_all("./archives/normal").unwrap();
}

#[test]
fn execute_infinite_loop() {
    setup();
    prepare_program("inf");
    let mut ch = CommandHandler::create();
    ch.execute_program("inf", "0002");
    while ch.is_program_running() {}

    std::fs::remove_dir_all("./archives/inf").unwrap();
}

#[test]
fn execute_multiple() {
    setup();
    prepare_program("multiple");
    let mut ch = CommandHandler::create();
    ch.execute_program("multiple", "0002");
    ch.execute_program("multiple", "0001");
    while ch.is_program_running() {}
    ch.execute_program("multiple", "0001");
    while ch.is_program_running() {}

    // TODO assertions (check log?)

    std::fs::remove_dir_all("./archives/multiple").unwrap();
}

#[test]
fn stop_program() {
    setup();
    prepare_program("stop");
    let mut ch = CommandHandler::create();
    ch.execute_program("stop", "0003");
    std::thread::sleep(std::time::Duration::from_millis(500));
    ch.stop_program();

    assert!(!ch.is_program_running());

    let mut res = String::new();
    std::fs::File::open("./archives/stop/results/0003/res.txt")
        .expect("res.txt not in results folder")
        .read_to_string(&mut res)
        .expect("Could not read res.txt");
    assert_eq!(res, *("First Line\n".to_string()));

    std::fs::remove_dir_all("./archives/stop").unwrap();
}