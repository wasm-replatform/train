//! # Provider
//!
//! Provider defines external data interfaces for the crate.

/// Provider entry point implemented by the host application.
pub trait Provider: Send + Sync {}

impl<T: Send + Sync> Provider for T {}
