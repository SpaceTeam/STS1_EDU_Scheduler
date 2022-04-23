use std::thread;
use std::sync;
use std::path;

pub struct CommunicationHandle {
    pub thread_handle: thread::JoinHandle<()>,
    /// Used to send filepaths which contain raw bytes to be sent to the COBC
    pub sender: sync::mpsc::Sender<path::PathBuf>, 
    /// Receives filepaths which contain raw bytes received from the COBC
    pub receiver: sync::mpsc::Receiver<path::PathBuf>
}

/// Spawns a thread which handles the communication with the CSBI
/// 
/// Returns a struct containing the threads JoinHandle and channels for message passing
pub fn spawn_communication_thread() -> CommunicationHandle {
    todo!();
}