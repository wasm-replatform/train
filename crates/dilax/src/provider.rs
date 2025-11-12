use std::any::Any;
use std::error::Error;
use std::future::Future;

use anyhow::Result;
use bytes::Bytes;
use chrono::Duration;
use http::{Request, Response};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + StateStore + Identity {}

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>;
}

/// The `StateStore` trait defines the behavior storing and retrieving train state.
pub trait StateStore: Send + Sync {
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    fn set(
        &self, key: &str, value: &[u8], expires: Option<Duration>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    fn delete(&self, key: &str) -> impl Future<Output = Result<()>> + Send;
}

pub trait Identity: Send + Sync {
    /// Get the unique identifier for the entity.
    fn access_token(&self) -> impl Future<Output = Result<String>> + Send;
}
