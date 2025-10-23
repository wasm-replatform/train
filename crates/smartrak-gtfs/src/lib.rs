//! SmarTrak GTFS adapter domain library.
//! Port of legacy/at_smartrak_gtfs_adapter domain logic.

pub mod cache;
pub mod config;
mod data_access;
pub mod god_mode;
pub mod locks;
pub mod model;
pub mod processor;
pub mod provider;
pub mod rest;
pub mod service;
pub mod error;

pub use crate::config::Config;
pub use crate::model::events::{PassengerCountEvent, SmartrakEvent};
pub use crate::provider::AdapterProvider;
pub use crate::rest::{ApiResponse, RestError, RestService, VehicleInfoResponse};
pub use crate::error::{HttpError, Error};
pub use crate::service::{
    KafkaWorkflow, ProcessingOutcome, Processor, ProducedMessage, SerializedMessage,
};
