//! # Provider
//!
//! Provider defines external data interfaces for the crate.



pub use realtime::{ Message, Publisher};

/// Provider entry point implemented by the host application.
pub trait Provider: Publisher {}

impl<T: Publisher> Provider for T {}

