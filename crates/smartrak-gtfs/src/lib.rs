//! SmarTrak GTFS adapter domain logic.

mod block_mgt;
mod fleet;
mod god_mode;
mod handlers;
mod location;
mod models;
pub mod rest;
mod serial_data;
mod trip;

pub use god_mode::*;
pub use handlers::*;
pub use models::*;

/// Result type for handlers.
pub use realtime::{Error, HttpRequest, Identity, Message, Publisher, Result, StateStore};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + StateStore + Identity {}
