use crate::communication::CommunicationError;

#[derive(Debug)]
pub enum CommandError {
    /// Propagates an error from the communication module
    CommunicationError(CommunicationError),
    /// Signals that something has gone wrong while using a system tool (e.g. unzip)
    SystemError(Box<dyn std::error::Error>),
    /// Signals that packets that are not useful right now were received
    InvalidCommError,
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<subprocess::PopenError> for CommandError {
    fn from(e: subprocess::PopenError) -> Self {
        CommandError::SystemError(e.into())
    }
}

impl From<CommunicationError> for CommandError {
    fn from(e: CommunicationError) -> Self {
        CommandError::CommunicationError(e)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CommandError {}
