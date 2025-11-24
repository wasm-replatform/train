use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
    #[error("code: processing_error, description: {0}")] 
    ProcessingError(String),
    #[error("code: invalid_format, description: {0}")] 
    InvalidFormat(String),
    #[error("code: outdated, description: {0}")] 
    Outdated(String),
    #[error("code: wrong_time, description: {0}")] 
    WrongTime(String),
    #[error("code: server_error, description: {0}")] 
    ServerError(String),
    #[error("code: no_update")] 
    NoUpdate,
    #[error("code: no_actual_update")] 
    NoActualUpdate,
}

impl Error {
    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::ProcessingError(_) => "processing_error",
            Self::InvalidFormat(_) => "invalid_format",
            Self::Outdated(_) => "outdated",
            Self::WrongTime(_) => "wrong_time",
            Self::ServerError(_) => "server_error",
            Self::NoUpdate => "no_update",
            Self::NoActualUpdate => "no_actual_update",
        }
    }
    #[must_use]
    pub fn description(&self) -> String { self.to_string() }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast_ref::<Self>() {
            Some(Self::ProcessingError(e)) => Self::ProcessingError(format!("{err}: {e}")),
            Some(Self::InvalidFormat(e)) => Self::InvalidFormat(format!("{err}: {e}")),
            Some(Self::Outdated(e)) => Self::Outdated(format!("{err}: {e}")),
            Some(Self::WrongTime(e)) => Self::WrongTime(format!("{err}: {e}")),
            Some(Self::ServerError(e)) => Self::ServerError(format!("{err}: {e}")),
            Some(Self::NoUpdate) => Self::NoUpdate,
            Some(Self::NoActualUpdate) => Self::NoActualUpdate,
            None => {
                let stack = err.chain().fold(String::new(), |cause, e| format!("{cause} -> {e}"));
                let stack = stack.trim_start_matches(" -> ").to_string();
                Self::ServerError(stack)
            }
        }
    }
}
