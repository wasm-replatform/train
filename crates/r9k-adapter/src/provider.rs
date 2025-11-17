//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}

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

#[derive(Clone, Debug)]
pub struct Message {
    pub payload: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl Message {
    #[must_use]
    pub fn new(payload: &[u8]) -> Self {
        Self { payload: payload.to_vec(), headers: HashMap::new() }
    }
}

/// The `Publisher` trait defines the message publishing behavior.
pub trait Publisher: Send + Sync {
    /// Make outbound HTTP request.
    fn send(&self, topic: &str, message: &Message) -> impl Future<Output = Result<()>> + Send;
}
