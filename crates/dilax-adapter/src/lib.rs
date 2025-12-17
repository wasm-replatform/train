//! Dilax domain library

mod gtfs;
mod handlers;
mod trip_state;
mod types;

pub use realtime::{Config, Error, HttpRequest, Identity, Message, Publisher, Result, StateStore};

pub use self::handlers::detector::*;
pub use self::handlers::processor::*;
pub use self::trip_state::*;
pub use self::types::*;

/// Provider entry point implemented by the host application.
pub trait Provider: Config + HttpRequest + Publisher + StateStore + Identity {}
impl<T> Provider for T where T: Config + HttpRequest + Publisher + StateStore + Identity {}
