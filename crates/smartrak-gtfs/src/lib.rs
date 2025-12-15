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

// use thiserror::Error;
// use realtime::Error as RealtimeError;

// /// SmarTrak GTFS specific errors
// #[derive(Error, Debug)]
// pub(crate) enum GtfsError {
//     /// GTFS feed not found
//     #[error("GTFS feed not found: {feed_id}")]
//     FeedNotFound { feed_id: String },

//     /// Invalid trip reference
//     #[error("Trip {trip_id} not found in GTFS data")]
//     TripNotFound { trip_id: String },

//     /// Invalid stop reference
//     #[error("Stop {stop_id} not found in GTFS data")]
//     StopNotFound { stop_id: String },

//     /// Schedule mismatch
//     #[error("Schedule mismatch for trip {trip_id}: {reason}")]
//     ScheduleMismatch {
//         trip_id: String,
//         reason: String,
//     },

//     /// Real-time update conflict
//     #[error("Update conflict for vehicle {vehicle_id}: {reason}")]
//     UpdateConflict {
//         vehicle_id: String,
//         reason: String,
//     },
// }

// impl From<GtfsError> for RealtimeError {
//     fn from(err: GtfsError) -> Self {
//         use GtfsError::*;
//         match err {
//             FeedNotFound { .. } | TripNotFound { .. } | StopNotFound { .. } => {
//                 Self::NotFound(err.to_string())
//             }
//             ScheduleMismatch { .. } => Self::BadRequest(err.to_string()),
//             UpdateConflict { .. } => Self::ServerError(err.to_string()),
//         }
//     }
// }
