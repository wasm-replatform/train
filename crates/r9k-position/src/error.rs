
//! # R9K Errors

use quick_xml::DeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `OpenID` error codes for  for Verifiable Credential Issuance and
/// Presentation.
#[derive(Error, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
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
    /// Returns the error code.
    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::InvalidFormat(_) => "invalid_format",
            Self::Outdated(_) => "outdated",
            Self::WrongTime(_) => "wrong_time",
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
            Some(Self::InvalidFormat(e)) => Self::InvalidFormat(format!("{err}: {e}")),
            Some(Self::Outdated(e)) => Self::Outdated(format!("{err}: {e}")),
            Some(Self::WrongTime(e)) => Self::WrongTime(format!("{err}: {e}")),
            Some(Self::ServerError(e)) => Self::ServerError(format!("{err}: {e}")),

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

impl From<DeError> for Error {
    fn from(err: DeError) -> Self {
        Self::InvalidFormat(format!("failed to deserialize message: {err}"))
    }
}

// /// Construct an `Error::InvalidRequest` error from a string or existing error
// /// value.
// macro_rules! invalid {
//     ($fmt:expr, $($arg:tt)*) => {
//         $crate::error::Error::InvalidRequest(format!($fmt, $($arg)*))
//     };
//      ($err:expr $(,)?) => {
//         $crate::error::Error::InvalidRequest(format!($err))
//     };
// }
// pub(crate) use invalid;

#[cfg(test)]
mod test {
    use anyhow::{Context, Result, anyhow};
    use serde_json::Value;

    use super::*;

    // Test that error details are retuned as json.
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
