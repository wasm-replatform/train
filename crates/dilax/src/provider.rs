//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use anyhow::Result;
use async_trait::async_trait;
use http::{Request, Response};

/// Host-provided HTTP client abstraction used by provider implementations.
#[async_trait]
pub trait HttpRequest: Send + Sync {
    /// Make an outbound HTTP request and return the raw response payload.
    async fn fetch(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>>;
}
