#![allow(clippy::collapsible_if)]
use std::fs::File;
use std::sync::*;
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
    watchdog_tx: Option<mpsc::Sender<bool>>,
    watchdog_handle: Option<thread::JoinHandle<()>>,
    student_program_running: Arc<atomic::AtomicBool>
}

impl CommandHandler {
    pub fn create() -> CommandHandler {
        CommandHandler {watchdog_tx: None, watchdog_handle: None, student_program_running: Arc::new(atomic::AtomicBool::new(false))}
    }

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

        // Store bytes into temporary file
        let mut zip_file = match File::create("./data/tmp.zip") { // TODO tmp file in /tmp ?
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
            .arg("-o") // overwrite silently
            .arg("./data/tmp.zip")
            .arg("-d") // target directory
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
        // Stop already running thread
        if self.watchdog_handle.is_some() {
            if self.watchdog_tx.take().unwrap().send(true).is_ok() {
                log::warn!("Program already running! Terminating...");
                self.watchdog_handle.take().unwrap().join().unwrap();
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
        self.student_program_running.store(true, atomic::Ordering::Relaxed);
        
        let (tx, rx) = mpsc::channel();
        self.watchdog_tx = Some(tx);
        let wd_flag = Arc::clone(&self.student_program_running);

        self.watchdog_handle = Some(thread::spawn(move || {
            // TODO proper timeout
            for _ in 0..2 {
                if student_process.poll().is_some() { // student program terminated itself
                    wd_flag.store(false, atomic::Ordering::Relaxed);
                    return;
                }
                if rx.try_recv().is_ok() { // check for restart/stop cmd
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }

            student_process.terminate().unwrap(); // SIGTERM
            if student_process.wait_timeout(Duration::from_millis(250)).unwrap().is_none() {
                log::warn!("Program not responding to SIGTERM, proceeding with SIGKILL");
                student_process.kill().unwrap(); // SIGKILL if still running
            }
            wd_flag.store(false, atomic::Ordering::Relaxed);
        }));
    }

    pub fn is_program_running(&self) -> bool {
        self.student_program_running.load(atomic::Ordering::Relaxed)
    }

    pub fn stop_program(&mut self) {
        log::info!("Stopping program...");
        if !self.is_program_running() {
            log::warn!("Attempting to stop program, but none is running");
            return;
        }

        if self.watchdog_tx.is_some() {
            self.watchdog_tx.as_ref().unwrap().send(true).expect("is_program_running() == true, but watchdog channel is dead");
            self.watchdog_handle.take()
                .expect("is_program_running() == true, but watchdog handle is none")
                .join().unwrap();
        }
        else {
            panic!("is_program_running() == true, but watchdog channel is none");
        }
    }
}
