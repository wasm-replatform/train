//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod handler;

pub use handler::*;
pub use realtime::{Error, Message, Publisher, Result};

/// Provider entry point implemented by the host application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}
