//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod error;
mod handler;

pub use handler::*;

pub use self::error::Error;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;

pub use realtime::{Message, Publisher};

/// Provider entry point implemented by the host application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}
