use crate::communication::CommunicationError;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Non-recoverable: {0:?}")]
    NonRecoverable(anyhow::Error),
    #[error("External: {0:?}")]
    External(anyhow::Error),
    #[error("Protocol Violation: {0:?}")]
    ProtocolViolation(anyhow::Error),
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
            CommunicationError::TimedOut
            | CommunicationError::NotAcknowledged
            | CommunicationError::CepParsing(_) => CommandError::ProtocolViolation(e.into()),
            CommunicationError::Io(_) => CommandError::NonRecoverable(e.into()),
        }
    }
}
