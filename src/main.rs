use log;
use simplelog as sl;
use std::io::prelude::*;

mod communication;
mod command;
use command::Command;

fn main() {
    let _ = sl::WriteLogger::init(sl::LevelFilter::Info, sl::Config::default(), std::fs::File::create("log").unwrap());
    
    let mut com = communication::spawn_communication_thread();
    let mut exec: Option<command::ExecutionContext> = None;

    loop {
        let received_file = com.receiver.recv().expect("Communication thread failed"); // Block until command is received
        let received_com = match command::process_payload(received_file) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Could not process command: {}", e);
                continue;
            }
        };
        
        let ret = match &received_com {
            Command::StoreArchive(arch, bytes) => command::store_archive(&arch, bytes),
            Command::ExecuteProgram(program, queue) => command::execute_program(&mut exec, &program, &queue),
            Command::StopProgram => command::stop_program(&mut exec),
            Command::ReturnResults(program, queue) => command::return_results(&program, &queue),
            Command::ListFiles => command::list_files(),
            Command::UpdateTime(epoch) => command::update_time(*epoch)
        };

        match ret {
            Ok(f) => todo!(),
            Err(e) => {
                log::error!("Could not execute <{:?}> returned <{}>", received_com, e);
                
            },
        }
    }
}   
