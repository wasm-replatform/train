//! SmarTrak GTFS adapter domain library.

pub mod cache;
pub mod config;
mod data_access;
pub mod god_mode;
pub mod locks;
pub mod model;
pub mod processor;
pub mod provider;
pub mod service;

pub use crate::config::Config;
pub use crate::model::events::{PassengerCountEvent, SmartrakEvent};
pub use crate::provider::AdapterProvider;
pub use crate::service::{Processor, ProducedMessage};
