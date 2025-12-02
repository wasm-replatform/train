//! # Dilax APC connector
//!
//! Receives Dilax passenger count requests and forwards to the `realtime-dilax-apc.v1` topic.

mod handler;
mod types;

pub use handler::*;
pub use realtime::{Error, Message, Publisher, Result};
pub use types::*;

/// Provider entry point implemented by the guest application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}
