//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use std::any::Any;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};

/// The `Provider` trait is implemented by library users in order to provide
/// source data and caching services required by the application.
pub trait Provider: Source + Time + Clone + Send + Sync {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>;
}
