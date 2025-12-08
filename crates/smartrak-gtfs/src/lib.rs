//! SmarTrak GTFS adapter domain logic.

pub mod block_mgt;
pub mod fleet;
pub mod god_mode;
pub mod models;
pub mod processor;
pub mod rest;
pub mod trip;
pub mod workflow;

pub use god_mode::*;
pub use models::*;
/// Result type for handlers.
pub use realtime::{Error, HttpRequest, Identity, Message, Publisher, Result, StateStore};
pub use workflow::*;

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + StateStore + Identity {}
