#![allow(clippy::collapsible_if)]

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path;
use std::sync::*;
use std::thread;
use std::time::Duration;

/// An enum storing a command with its parameters
pub enum Command {
    StoreArchive(String, Vec<u8>),
    ExecuteProgram(String, String),
    StopProgram,
    ReturnResults(String, String),
    ListFiles,
    UpdateTime(u64),
}

pub struct StudentProgram {
    watchdog_tx: Option<mpsc::Sender<bool>>,
    watchdog_handle: Option<thread::JoinHandle<()>>,
    student_program_running: Arc<atomic::AtomicBool>,
}

/// Parse a command coming from the COBC
///
/// `data_path` A path to the file containing the received command
///
/// Returns the resulting command with parameters or passes along the IO Error from file access
pub fn process_payload(data_path: path::PathBuf) -> Result<Command, std::io::Error> {
    todo!();
}

/// Stores a received program in the appropriate folder and unzips it
///
/// * `folder` The folder to unzip into, subsequently the program id
/// * `bytes` A vector containing the raw bytes of the zip archive
///
/// Returns Ok or passes along a file access/unzip process error
pub fn store_archive(folder: &str, bytes: Vec<u8>) -> Result<(), std::io::Error> {
    // Store bytes into temporary file
    let mut zip_file = File::create("./data/tmp.zip")?;
    zip_file.write_all(&bytes)?;
    zip_file.sync_all()?;

    let exit_status = subprocess::Exec::cmd("unzip")
        .arg("-o") // overwrite silently
        .arg("./data/tmp.zip")
        .arg("-d") // target directory
        .arg(format!("./archives/{}", folder))
        .join();

    std::fs::remove_file("./data/tmp.zip")?;

    match exit_status {
        Ok(status) => {
            if !status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unzip returned with {:?}", status),
                ));
            }
        }
        Err(err) => match err {
            subprocess::PopenError::IoError(e) => return Err(e),
            _ => {
                unreachable!() // should only appear on Exec::cmd without args
            }
        },
    }

    Ok(())
}

/// Executes a students program and starts a watchdog for it
///
/// * `program_id` The name of the ./archives/ subfolder
/// * `queue_id` The first argument for the student program
pub fn execute_program(sp: &mut StudentProgram, program_id: &str, queue_id: &str) {
    // Stop already running thread
    if sp.watchdog_handle.is_some() {
        if sp.watchdog_tx.take().unwrap().send(true).is_ok() {
            log::warn!("Program already running! Terminating...");
            sp.watchdog_handle.take().unwrap().join().unwrap();
        }
    }

    log::info!("Executing program: {} with {}", program_id, queue_id);
    // TODO config setuid
    let config = subprocess::PopenConfig {
        cwd: Some(format!("./archives/{}", program_id).into()),
        ..Default::default()
    };
    let mut student_process =
        match subprocess::Popen::create(&["python", "main.py", queue_id], config) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Could not start student program: {}", e);
                return;
            }
        };
    sp.student_program_running
        .store(true, atomic::Ordering::Relaxed);

    let (tx, rx) = mpsc::channel();
    sp.watchdog_tx = Some(tx);
    let wd_flag = Arc::clone(&sp.student_program_running);

    sp.watchdog_handle = Some(thread::spawn(move || {
        // TODO proper timeout
        for _ in 0..2 {
            if student_process.poll().is_some() {
                // student program terminated itself
                wd_flag.store(false, atomic::Ordering::Relaxed);
                return;
            }
            if rx.try_recv().is_ok() {
                // check for restart/stop cmd
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }

        student_process.terminate().unwrap(); // SIGTERM
        if student_process
            .wait_timeout(Duration::from_millis(250))
            .unwrap()
            .is_none()
        {
            log::warn!("Program not responding to SIGTERM, proceeding with SIGKILL");
            student_process.kill().unwrap(); // SIGKILL if still running
        }
        wd_flag.store(false, atomic::Ordering::Relaxed);
    }));
}

pub fn stop_program(sp: &mut StudentProgram) {
    log::info!("Stopping program...");

    if sp.watchdog_tx.is_some() {
        sp.watchdog_tx
            .as_ref()
            .unwrap()
            .send(true)
            .expect("is_program_running() == true, but watchdog channel is dead");
        sp.watchdog_handle
            .take()
            .expect("is_program_running() == true, but watchdog handle is none")
            .join()
            .unwrap();
    } else {
        panic!("is_program_running() == true, but watchdog channel is none");
    }
}
