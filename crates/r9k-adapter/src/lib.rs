//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod handler;
mod r9k;
mod smartrak;
mod stops;

use thiserror::Error;
use warp_sdk::Error;

pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

// TODO: use for internal methods
#[derive(Error, Debug)]
pub enum R9kError {
    /// The message timestamp is invalid (too old or future-dated).
    #[error("{0}")]
    BadTime(String),

    /// The message contains no updates or the arrival/departure time is
    /// invalid (negative or 0).
    #[error("{0}")]
    NoUpdate(String),

    /// The XML is invalid.
    #[error("{0}")]
    InvalidXml(String),
}

impl R9kError {
    fn code(&self) -> String {
        match self {
            Self::BadTime(_) => "bad_time".to_string(),
            Self::NoUpdate(_) => "no_update".to_string(),
            Self::InvalidXml(_) => "invalid_message".to_string(),
        }
    }
}

impl From<R9kError> for Error {
    fn from(err: R9kError) -> Self {
        Self::BadRequest { code: err.code(), description: err.to_string() }
    }
}

impl From<quick_xml::DeError> for R9kError {
    fn from(err: quick_xml::DeError) -> Self {
        Self::InvalidXml(err.to_string())
    }
}
