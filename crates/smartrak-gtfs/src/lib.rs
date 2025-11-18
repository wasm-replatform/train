//! SmarTrak GTFS adapter domain logic.

pub mod block_mgt;
pub mod error;
pub mod fleet;
pub mod god_mode;
pub mod key_locker;
pub mod models;
pub mod processor;
pub mod rest;
pub mod trip;
pub mod workflow;

pub use error::*;
pub use god_mode::*;
pub use key_locker::*;
pub use models::*;
pub use workflow::*;


/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;

pub use realtime::{HttpRequest, Identity, Message, Publisher, StateStore};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: HttpRequest + Publisher + StateStore + Identity {}
