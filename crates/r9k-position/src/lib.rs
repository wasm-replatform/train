//! # R9K  Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod error;
mod handler;
mod r9k;
mod smartrak;
mod stops;

pub mod gtfs;
pub mod provider;
pub mod r9k_date;

pub use self::error::Error;
pub use self::handler::R9kResponse;
pub use self::r9k::*;
pub use self::smartrak::*;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;
