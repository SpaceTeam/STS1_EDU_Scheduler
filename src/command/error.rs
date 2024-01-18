use crate::communication::CommunicationError;

type BoxedError = Box<dyn std::error::Error>;

#[derive(Debug)]
pub enum CommandError {
    NonRecoverable(BoxedError),
    External(BoxedError),
    ProtocolViolation(BoxedError),
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::NonRecoverable(e.into())
    }
}

impl From<subprocess::PopenError> for CommandError {
    fn from(e: subprocess::PopenError) -> Self {
        CommandError::NonRecoverable(e.into())
    }
}

impl From<CommunicationError> for CommandError {
    fn from(e: CommunicationError) -> Self {
        match e {
            CommunicationError::PacketInvalidError => CommandError::External(e.into()),
            CommunicationError::CepParsing(_) => CommandError::ProtocolViolation(e.into()),
            CommunicationError::Io(_) => CommandError::NonRecoverable(e.into()),
            CommunicationError::NotAcknowledged => CommandError::ProtocolViolation(e.into()),
            CommunicationError::TimedOut => CommandError::ProtocolViolation(e.into()),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandError::{:?}", self)
    }
}

impl std::error::Error for CommandError {}
