//! # R9K Transformer
//!
//! Transforms R9K messages into SmarTrak events.

mod handler;
mod r9k;
mod smartrak;
mod stops;

pub use realtime::{Config, Error, HttpRequest, Identity, Message, Publisher, Result};

pub use self::r9k::*;
pub use self::smartrak::*;
pub use self::stops::StopInfo;

/// Provider entry point implemented by the host application.
pub trait Provider: Config + HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: Config + HttpRequest + Identity + Publisher {}
