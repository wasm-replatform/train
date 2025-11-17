//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use std::collections::HashMap;

use anyhow::Result;

/// Provider entry point implemented by the host application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}

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

/// The `Publisher` trait defines the message publishing behavior required.
pub trait Publisher: Send + Sync {
    /// Make outbound HTTP request.
    fn send(&self, topic: &str, message: &Message) -> impl Future<Output = Result<()>> + Send;
}
