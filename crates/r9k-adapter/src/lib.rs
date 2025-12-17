//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod handler;
mod r9k;
mod smartrak;
mod stops;

use realtime::Error;
use thiserror::Error;

pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

// TODO: use for internal methods
#[derive(Error, Debug)]
enum R9kError {
    /// The message timestamp is invalid (too old or future-dated).
    #[error("{0}")]
    BadTime(String),

    /// The message contains no updates or the arrival/departure time is
    /// invalid (negative or 0).
    #[error("{0}")]
    NoUpdate(String),
}

impl R9kError {
    fn code(&self) -> String {
        match self {
            Self::BadTime(_) => "bad_time".to_string(),
            Self::NoUpdate(_) => "no_update".to_string(),
        }
    }
}

impl From<R9kError> for Error {
    fn from(err: R9kError) -> Self {
        Self::BadRequest { code: err.code(), description: err.to_string() }
    }
}
