//! # Dilax APC connector
//!
//! Receives Dilax passenger count requests and forwards to the `realtime-dilax-apc.v2` topic.

mod handler;
mod types;

pub use handler::*;
pub use types::*;
