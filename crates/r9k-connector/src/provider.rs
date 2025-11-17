//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use anyhow::Result;

/// Provider entry point implemented by the host application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait Publisher: Send + Sync {
    /// Make outbound HTTP request.
    fn send(&self, topic: &str, payload: &[u8]) -> impl Future<Output = Result<()>> + Send;
}
