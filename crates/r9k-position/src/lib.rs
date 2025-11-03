
//! # R9K  Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod error;
mod handler;
mod provider;
mod r9k;
mod smartrak;
mod stops;

pub use self::error::Error;
pub use self::handler::R9kResponse;
pub use self::provider::{HttpRequest, Provider};
pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;
