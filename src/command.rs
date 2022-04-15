#![allow(clippy::collapsible_if)]
use std::fs::File;
use std::sync;
use std::thread;
use std::time::Duration;
use std::io::prelude::*;

pub enum Command {
    StoreArchive,
    ExecuteProgram,
    StopProgram,
    ReturnResults,
    ListFiles,
    UpdateTime,
}

pub struct CommandHandler {
    watchdog_tx: Option<sync::mpsc::Sender<bool>>,
    watchdog_handle: Option<thread::JoinHandle<()>>,
}

impl CommandHandler {
    /// Dispatches a command to the appropriate handler with preprocessed data
    ///
    /// * `cmd` The received command
    /// * `path` A path to a file with the received command data
    pub fn dispatch_command(cmd: Command, path: String) {
        todo!();
    }

    /// Stores a received program in the appropriate folder and unzips it
    ///
    /// * `folder` The folder to unzip into, subsequently the program id
    /// * `bytes` A vector containing the raw bytes of the zip archive
    pub fn store_archive(folder: &str, bytes: Vec<u8>) {
        log::info!("Store Archive: {}", folder);
        let mut zip_file = match File::create("./data/tmp.zip") {
            Ok(f) => f,
            Err(e) => {
                log::error!("Could not create zipfile: {}", e);
                return;
            }
        };
        match zip_file.write_all(&bytes).and(zip_file.flush()) {
            Ok(_) => (),
            Err(e) => {
                log::error!("Could not write zipfile: {}", e);
                return;
            },
        }
        match subprocess::Exec::cmd("unzip")
            .arg("-o")
            .arg("./data/tmp.zip")
            .arg("-d")
            .arg(format!("./archives/{}", folder))
            .join()
        {
            Ok(es) => if !es.success() {
                log::error!("Unzip returned with: {:?}", es);
                return;
            }
            Err(e) => {
                log::error!("Unzip could not be started {}", e);
                return;
            }
        }
        std::fs::remove_file("./data/tmp.zip").unwrap();
    }

    /// Executes a students program and starts a watchdog for it
    /// 
    /// * `program_id` The name of the ./archives/ subfolder
    /// * `queue_id` The first argument for the student program
    pub fn execute_program(&mut self, program_id: &str, queue_id: &str) {
        if self.watchdog_handle.is_some() {
            if self.watchdog_tx.take().unwrap().send(true).is_ok() {
                log::warn!("Program already running! Terminating...");
                self.watchdog_handle.take().unwrap().join();
            }
        }

        log::info!("Executing program: {} with {}", program_id, queue_id);
        // TODO config setuid
        let config = subprocess::PopenConfig {
            cwd: Some(format!("./archives/{}", program_id).into()),
            ..Default::default()
        };
        let mut student_process = match subprocess::Popen::create(&["python", "main.py", queue_id], config) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Could not start student program: {}", e);
                return;
            }
        };
        
        let (tx, rx) = sync::mpsc::channel();
        self.watchdog_tx = Some(tx);

        self.watchdog_handle = Some(thread::spawn(move || {
            // TODO proper timeout
            for _ in 0..2 {
                if rx.try_recv().is_ok() {
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }
            student_process.terminate().unwrap();
            // Allow cleanup, kill if it blocks
            if student_process.wait_timeout(Duration::from_millis(250)).unwrap().is_none() {
                student_process.kill().unwrap();
            }

        }));
    }
}
