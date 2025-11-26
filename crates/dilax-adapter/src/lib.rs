//! Dilax domain library

mod block_mgt;
mod error;
mod gtfs;
mod trip_state;
pub mod handlers;
pub mod types;

pub use self::error::Error;
pub use self::handlers::detector::*;
pub use self::handlers::processor::*;
pub use self::trip_state::*;
pub use self::types::*;

pub use realtime::{HttpRequest, Identity, Message, Publisher, StateStore};

pub type Result<T> = std::result::Result<T, crate::Error>;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + StateStore + Identity {}
