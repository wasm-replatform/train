//! # Cars adapter errors

// use serde::{Deserialize, Serialize};
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use thiserror::Error;

/// Result type used across the crate.
pub type Result<T> = anyhow::Result<T, Error>;

/// Domain level error type returned by the adapter.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The request payload is invalid or missing required fields.
    #[error("code: 400, error: {0}")]
    BadRequest(String),

    /// The request is not authenticated.
    #[error("code: 401, error: {0}")]
    Unauthorized(String),

    /// The request is unauthorized.
    #[error("code: 403, error: {0}")]
    Forbidden(String),

    /// The requested resource could not be found.
    #[error("code: 404, error: {0}")]
    NotFound(String),

    /// A non recoverable internal error occurred.
    #[error("code: 500, error: {0}")]
    Internal(String),

    /// An upstream dependency failed while fulfilling the request.
    #[error("code: 502, error: {0}")]
    BadGateway(String),
}

impl Error {
    /// Returns the stable error code associated with the variant.
    #[must_use]
    pub const fn code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        let chain = err.chain().map(ToString::to_string).collect::<Vec<_>>().join(" >> ");

        // if type is Error, return it with the newly added context
        if let Some(inner) = err.downcast_ref::<Self>() {
            tracing::debug!("Error: {err}, caused by: {inner}");

            return match inner {
                Self::BadRequest(_s) => Self::BadRequest(chain),
                Self::NotFound(_s) => Self::NotFound(chain),
                Self::Unauthorized(_s) => Self::Unauthorized(chain),
                Self::Forbidden(_s) => Self::Forbidden(chain),
                Self::BadGateway(_s) => Self::BadGateway(chain),
                Self::Internal(_s) => Self::Internal(chain),
            };
        }

        // otherwise, return an Internal error
        Self::Internal(chain)
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
    use anyhow::Context;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Registry, fmt};

    use super::Error;

    #[test]
    fn error_display() {
        let err = Error::BadRequest("invalid input".to_string());
        assert_eq!(format!("{err}",), "code: 400, error: invalid input");
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
                "more context >> doing something >> code: 400, error: invalid input".to_string()
            )
        );
    }
}
