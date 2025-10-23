//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use anyhow::Result;
use http::{Request, Response};
use serde::de::DeserializeOwned;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<B: Sync, U: DeserializeOwned>(
        &self, request: &Request<B>,
    ) -> impl Future<Output = Result<Response<U>>> + Send;
}
