use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain-level errors that should be surfaced to downstream consumers.
#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainError {
    #[error("code: invalid_event, description: {0}")]
    InvalidEvent(String),
    #[error("code: processing_error, description: {0}")]
    ProcessingError(String),
    #[error("code: state_conflict, description: {0}")]
    StateConflict(String),
}

/// Adapter error type that separates domain failures from infrastructure failures.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    System(#[from] anyhow::Error),
}

/// Result alias for adapter APIs.
pub type Result<T> = std::result::Result<T, Error>;
