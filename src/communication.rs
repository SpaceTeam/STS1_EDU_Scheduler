use std::thread;
use std::sync;
use std::path;

pub trait Communication {
    /// Sends the bytes in the file to the COBC
    fn send(&mut self, p: path::PathBuf);
    /// Blocks until a command from the COBC is received. Returns its file
    fn receive(&self) -> path::PathBuf;
}

pub struct CommunicationHandle {
    pub thread_handle: thread::JoinHandle<()>,
    /// Used to send filepaths which contain raw bytes to be sent to the COBC
    pub sender: sync::mpsc::Sender<path::PathBuf>, 
    /// Receives filepaths which contain raw bytes received from the COBC
    pub receiver: sync::mpsc::Receiver<path::PathBuf>
}

impl Communication for CommunicationHandle {
    fn send(&mut self, p: path::PathBuf) {
        todo!();
    }

    fn receive(&self) -> path::PathBuf {
        todo!();
    }
}

/// Spawns a thread which handles the communication with the CSBI
/// 
/// Returns a struct containing the threads JoinHandle and channels for message passing
pub fn spawn_communication_thread() -> CommunicationHandle {
    todo!();
}