//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use std::any::Any;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Identity {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>;
}

pub trait Identity: Send + Sync {
    /// Get the unique identifier for the entity.
    fn access_token(&self) -> impl Future<Output = Result<String>> + Send;
}
