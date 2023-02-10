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
            CommunicationError::PacketInvalidError => CommandError::External(Box::new(e)),
            CommunicationError::CRCError => CommandError::ProtocolViolation(Box::new(e)),
            CommunicationError::InterfaceError => CommandError::NonRecoverable(Box::new(e)),
            CommunicationError::STOPCondition => CommandError::External(Box::new(e)),
            CommunicationError::TimeoutError => todo!("Timeout not yet specified"),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandError::{:?}", self)
    }
}

impl std::error::Error for CommandError {}
