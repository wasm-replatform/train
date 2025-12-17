//! # R9K HTTP Connector
//!
//! Processes R9K SOAP requests and forwards to the `r9k-adapter` topic.

mod handler;

pub use handler::*;
