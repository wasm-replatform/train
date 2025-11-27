//! Dilax domain library

mod block_mgt;
mod error;
mod gtfs;
mod handlers;
mod trip_state;
mod types;

pub use self::error::Error;
pub use self::handlers::detector::*;
pub use self::handlers::processor::*;
pub use self::trip_state::*;
pub use self::types::*;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;

pub use realtime::{Config, HttpRequest, Identity, Message, Publisher, StateStore};

/// Provider entry point implemented by the host application.
pub trait Provider: Config + HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: Config + HttpRequest + Publisher + StateStore + Identity {}
