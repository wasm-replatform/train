//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod handler;
mod provider;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, anyhow::Error>;
