//! SmarTrak GTFS adapter.

mod god_mode;
mod handlers;
mod location;
// pub mod rest;
mod serial_data;
mod trip;

use fabric::Error;
pub use god_mode::*;
pub use handlers::*;
use thiserror::Error;

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
