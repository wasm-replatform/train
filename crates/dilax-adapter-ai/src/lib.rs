//! Dilax Adapter
//!
//! Provides passenger count enrichment and lost connection detection for Dilax events.

pub mod config;
pub mod error;
pub mod handlers;
pub mod logic;
pub mod provider;
pub mod types;

pub use crate::config::Config;
pub use crate::error::{DomainError, Error, Result};
pub use crate::handlers::{DetectionRequest, DetectionResponse, DilaxMessage, DilaxResponse};
pub use crate::logic::{detect_lost_connections, fetch_allocations_for_today, process_event};
pub use crate::provider::{Provider, ProviderWrapper};
