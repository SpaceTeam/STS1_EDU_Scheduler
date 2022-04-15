use STS1_EDU_Scheduler::command::*;
use std::io::prelude::*;
use simplelog as sl;


fn setup() {
    let _ = sl::SimpleLogger::init(sl::LevelFilter::Info, sl::Config::default());
}

fn prepare_program() {
    std::fs::create_dir("./archives/testa").unwrap();
    std::fs::copy("./tests/test_data/main.py", "./archives/testa/main.py").unwrap();
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
    prepare_program();
    todo!();
}