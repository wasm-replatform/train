//! SmarTrak GTFS adapter.

mod god_mode;
mod handlers;
mod location;
pub mod rest;
mod serial_data;
mod trip;

pub use god_mode::*;
pub use handlers::*;
pub use realtime::{Config, Error, HttpRequest, Identity, Message, Publisher, Result, StateStore};
use thiserror::Error;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + StateStore + Identity + Config {}
impl<T> Provider for T where T: HttpRequest + Publisher + StateStore + Identity + Config {}

// TODO: use for internal methods
#[derive(Error, Debug)]
enum SmarTrakError {
    /// The message timestamp is invalid (too old or future-dated).
    #[error("{0}")]
    BadTime(String),
}

impl SmarTrakError {
    fn code(&self) -> String {
        match self {
            Self::BadTime(_) => "bad_time".to_string(),
        }
    }
}

impl From<SmarTrakError> for Error {
    fn from(err: SmarTrakError) -> Self {
        Self::BadRequest { code: err.code(), description: err.to_string() }
    }
}
