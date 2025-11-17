//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod error;
mod handler;
mod r9k;
mod smartrak;
mod stops;

pub use self::error::Error;
pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

/// Result type for handlers.
pub type Result<T> = anyhow::Result<T, Error>;

pub use realtime::{HttpRequest, Identity, Message, Publisher};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}
