use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain-specific error codes for SmarTrak GTFS adapter processing.
/// Includes data format, missing field, timestamp, caching, server, and update errors.
#[derive(Error, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
    #[error("code: processing_error, description: {0}")]
    ProcessingError(String),

    #[error("code: invalid_format, description: {0}")]
    InvalidFormat(String),

    #[error("code: missing_field, description: missing {0}")]
    MissingField(String),

    #[error("code: invalid_timestamp, description: {0}")]
    InvalidTimestamp(String),

    #[error("code: outdated, description: {0}")]
    Outdated(String),

    #[error("code: wrong_time, description: {0}")]
    WrongTime(String),

    #[error("code: caching_error, description: {0}")]
    CachingError(String),

    #[error("code: server_error, description: {0}")]
    ServerError(String),

    #[error("code: no_update")]
    NoUpdate,

    #[error("code: no_actual_update")]
    NoActualUpdate,
}

impl Error {
    /// Returns the error code.
    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::ProcessingError(_) => "processing_error",
            Self::InvalidFormat(_) => "invalid_format",
            Self::MissingField(_) => "missing_field",
            Self::InvalidTimestamp(_) => "invalid_timestamp",
            Self::Outdated(_) => "outdated",
            Self::WrongTime(_) => "wrong_time",
            Self::CachingError(_) => "caching_error",
            Self::ServerError(_) => "server_error",
            Self::NoUpdate => "no_update",
            Self::NoActualUpdate => "no_actual_update",
        }
    }

    /// Returns the error description.
    #[must_use]
    pub fn description(&self) -> String {
        self.to_string()
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast_ref::<Self>() {
            Some(Self::ProcessingError(e)) => Self::ProcessingError(format!("{err}: {e}")),
            Some(Self::InvalidFormat(e)) => Self::InvalidFormat(format!("{err}: {e}")),
            Some(Self::MissingField(e)) => Self::MissingField(format!("{err}: {e}")),
            Some(Self::InvalidTimestamp(e)) => Self::InvalidTimestamp(format!("{err}: {e}")),
            Some(Self::Outdated(e)) => Self::Outdated(format!("{err}: {e}")),
            Some(Self::WrongTime(e)) => Self::WrongTime(format!("{err}: {e}")),
            Some(Self::ServerError(e)) => Self::ServerError(format!("{err}: {e}")),
            Some(Self::CachingError(e)) => Self::CachingError(format!("{err}: {e}")),
            // Handle the specific cases for NoUpdate and NoActualUpdate
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

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidFormat(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use anyhow::{Context, Result, anyhow};
    use serde_json::Value;

    use super::*;

    // Test that error details are returned as json.
    #[test]
    fn r9k_context() {
        let result = Err::<(), Error>(Error::ServerError("server error".to_string()))
            .context("request context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(
            err.to_string(),
            "code: server_error, description: request context: server error"
        );
    }

    #[test]
    fn anyhow_context() {
        let result = Err::<(), anyhow::Error>(anyhow!("one-off error")).context("error context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(
            err.to_string(),
            "code: server_error, description: error context -> one-off error"
        );
    }

    #[test]
    fn serde_context() {
        let result: Result<Value, anyhow::Error> =
            serde_json::from_str(r#"{"foo": "bar""#).context("error context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(
            err.to_string(),
            "code: server_error, description: error context -> EOF while parsing an object at line 1 column 13"
        );
    }

    // // Test that the error details are returned as an http query string.
    // #[test]
    // fn json() {
    //     let err = Error::ServerError("bad request".to_string());
    //     let ser = serde_json::to_value(&err).unwrap();
    //     assert_eq!(ser, json!({"code": "server_error", "description": "bad request"}));
    // }
}
