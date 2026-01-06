//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod handler;

pub use handler::*;
use thiserror::Error;
use warp_sdk::Error;

// TODO: use for internal methods
#[derive(Error, Debug)]
pub enum R9kError {
    /// The XML is invalid.
    #[error("{0}")]
    InvalidXml(String),
}

impl R9kError {
    fn code(&self) -> String {
        match self {
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
