//! R9K position adapter domain library

mod error;
mod types;
mod config;
mod block_mgt;
mod gtfs;
mod handlers;

pub use self::error::Error;
pub use self::types::{R9kMessage, SmarTrakEvent};
pub use self::gtfs::StopInfo;
pub use self::handlers::processor::process;

pub use realtime::{HttpRequest, Identity, Message, Publisher, StateStore};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + Identity {}

/// Result type for r9k adapter handlers.
pub type Result<T> = anyhow::Result<T, Error>;
