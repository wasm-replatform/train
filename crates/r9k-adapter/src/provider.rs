//! # Provider
//!
//! Provider defines external data interfaces for the crate.

pub use realtime::{HttpRequest, Identity, Message, Publisher};

/// Provider entry point implemented by the host application.
pub trait Provider: HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}
