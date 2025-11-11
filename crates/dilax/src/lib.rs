//! Dilax domain library

mod block_mgt;
mod error;
mod gtfs;
mod handlers;
mod provider;
mod state;
mod types;

pub use self::error::Error;
pub use self::handlers::detector::*;
pub use self::handlers::processor::*;
pub use self::provider::*;
pub use self::state::*;
pub use self::types::*;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;
