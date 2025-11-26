//! R9K position adapter domain library

mod block_mgt;
mod config;
mod error;
mod gtfs;
mod handlers;
mod types;

pub use self::error::Error;
pub use self::handlers::processor::*;
pub use self::types::*;

pub use realtime::{HttpRequest, Identity, Message, Publisher, StateStore};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + Identity {}

/// Result type for r9k adapter handlers.
pub type Result<T> = anyhow::Result<T, Error>;
