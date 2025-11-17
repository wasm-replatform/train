//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod error;
mod handler;
mod provider;

pub use handler::*;

pub use self::error::Error;
pub use self::provider::*;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;
