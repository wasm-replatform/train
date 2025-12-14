//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod handler;
mod r9k;
mod smartrak;
mod stops;

pub use realtime::{Config, Error, HttpRequest, Identity, Message, Publisher, Result};
use thiserror::Error;

pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

/// Provider entry point implemented by the host application.
pub trait Provider: Config + HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: Config + HttpRequest + Identity + Publisher {}

#[derive(Error, Debug)]
enum R9kError {
    /// The message is outdated or ahead of time.
    #[error("code: bad_time, description: {0}")]
    BadTime(String),

    /// The message has no updates or arrival/departure time <= 0.
    #[error("code: no_update, description: {0}")]
    NoUpdate(String),
}

impl From<R9kError> for Error {
    fn from(err: R9kError) -> Self {
        Self::BadRequest(err.to_string())
    }
}
