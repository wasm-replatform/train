//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod error;
mod handler;
mod provider;
mod r9k;

pub use self::error::Error;
pub use self::provider::*;
pub use self::r9k::*;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;
