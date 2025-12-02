//! Trains service errors

use axum::response::{IntoResponse, Response};
use http::StatusCode;
use quick_xml::DeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type used across the crate.
pub type Result<T> = anyhow::Result<T, Error>;

/// Domain level error type returned by the adapter.
#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
    /// The request payload is invalid or missing required fields.
    #[error("code: 400, description: {0}")]
    BadRequest(String),

    /// The requested resource could not be found.
    #[error("code: 404, description: {0}")]
    NotFound(String),

    /// A non recoverable internal error occurred.
    #[error("code: 500, description: {0}")]
    Internal(String),

    /// An upstream dependency failed while fulfilling the request.
    #[error("code: 502, description: {0}")]
    BadGateway(String),

    /// A processing error occurred.
    #[error("code: 500, description: processing_error {0}")]
    ProcessingError(String),

    /// A processing error occurred.
    #[error("code: 500, description: invalid_format {0}")]
    InvalidFormat(String),

    /// A processing error occurred.
    #[error("code: 500, description: missing_field {0}")]
    MissingField(String),

    /// A processing error occurred.
    #[error("code: 500, description: invalid_timestamp {0}")]
    InvalidTimestamp(String),

    /// Delayed message arrival.
    #[error("code: 500, description: outdated {0}")]
    Outdated(String),

    /// Ahead of time message arrival.
    #[error("code: 500, description: wrong_time {0}")]
    WrongTime(String),

    /// A processing error occurred.
    #[error("code: 500, description: server_error {0}")]
    ServerError(String),

    /// A processing error occurred.
    #[error("code: 500, description: no_update")]
    NoUpdate,

    /// A processing error occurred.
    #[error("code: 500, description: no_actual_update")]
    NoActualUpdate,
}

impl Error {
    /// Returns the stable error code associated with the variant.
    #[must_use]
    pub const fn code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
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
        let chain = err.chain().map(ToString::to_string).collect::<Vec<_>>().join(" -> ");

        // if type is Error, return it with the newly added context
        if let Some(inner) = err.downcast_ref::<Self>() {
            tracing::debug!("Error: {err}, caused by: {inner}");

            return match inner {
                Self::BadRequest(_s) => Self::BadRequest(chain),
                Self::NotFound(_s) => Self::NotFound(chain),
                Self::BadGateway(_s) => Self::BadGateway(chain),
                Self::Internal(_s) => Self::Internal(chain),
                Self::ProcessingError(e) => Self::ProcessingError(format!("{err}: {e}")),
                Self::InvalidFormat(e) => Self::InvalidFormat(format!("{err}: {e}")),
                Self::MissingField(e) => Self::MissingField(format!("{err}: {e}")),
                Self::InvalidTimestamp(e) => Self::InvalidTimestamp(format!("{err}: {e}")),
                Self::Outdated(e) => Self::Outdated(format!("{err}: {e}")),
                Self::WrongTime(e) => Self::WrongTime(format!("{err}: {e}")),
                Self::ServerError(e) => Self::ServerError(format!("{err}: {e}")),
                // Handle the specific cases for NoUpdate and NoActualUpdate
                Self::NoUpdate => Self::NoUpdate,
                Self::NoActualUpdate => Self::NoActualUpdate,
            };
        }

        // otherwise, return an Internal error
        Self::Internal(chain)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidFormat(err.to_string())
    }
}

impl From<DeError> for Error {
    fn from(err: DeError) -> Self {
        Self::InvalidFormat(format!("failed to deserialize message: {err}"))
    }
}

pub struct HttpError {
    status: StatusCode,
    error: String,
}

impl From<anyhow::Error> for HttpError {
    fn from(e: anyhow::Error) -> Self {
        let error = format!("{e}, caused by: {}", e.root_cause());
        let status = e.downcast_ref().map_or(StatusCode::INTERNAL_SERVER_ERROR, Error::code);
        Self { status, error }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, self.error).into_response()
    }
}

#[macro_export]
macro_rules! bad_request {
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error::BadRequest(format!($fmt, $($arg)*))
    };
     ($err:expr $(,)?) => {
        $crate::Error::BadRequest(format!($err))
    };
}

#[macro_export]
macro_rules! not_found {
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error::NotFound(format!($fmt, $($arg)*))
    };
     ($err:expr $(,)?) => {
        $crate::Error::NotFound(format!($err))
    };
}

#[macro_export]
macro_rules! bad_gateway {
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error::BadGateway(format!($fmt, $($arg)*))
    };
     ($err:expr $(,)?) => {
        $crate::Error::BadGateway(format!($err))
    };
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result, anyhow};
    use serde_json::Value;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Registry, fmt};

    use super::Error;

    #[test]
    fn error_display() {
        let err = Error::BadRequest("invalid input".to_string());
        assert_eq!(format!("{err}",), "code: 400, description: invalid input");
    }

    #[test]
    fn with_context() {
        Registry::default().with(EnvFilter::new("debug")).with(fmt::layer()).init();

        let context_error = || -> Result<(), Error> {
            Err(Error::BadRequest("invalid input".to_string()))
                .context("doing something")
                .context("more context")?;
            Ok(())
        };

        let result = context_error();
        assert_eq!(
            result.unwrap_err(),
            Error::BadRequest(
                "more context -> doing something -> code: 400, description: invalid input"
                    .to_string()
            )
        );
    }

    // Test that error details are returned as json.
    #[test]
    fn r9k_context() {
        let result = Err::<(), Error>(Error::ServerError("server error".to_string()))
            .context("request context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(
            err.to_string(),
            "code: 500, description: server_error request context: server error"
        );
    }

    #[test]
    fn anyhow_context() {
        let result = Err::<(), anyhow::Error>(anyhow!("one-off error")).context("error context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(err.to_string(), "code: 500, description: error context -> one-off error");
    }

    #[test]
    fn serde_context() {
        let result: Result<Value, anyhow::Error> =
            serde_json::from_str(r#"{"foo": "bar""#).context("error context");
        let err: Error = result.unwrap_err().into();

        assert_eq!(
            err.to_string(),
            "code: 500, description: error context -> EOF while parsing an object at line 1 column 13"
        );
    }
}
