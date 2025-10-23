//! # Provider
//!
//! The `Provider` module defines the traits required to integrate
//! external (infrastructural) services into the application.

use anyhow::Result;
use async_trait::async_trait;

use crate::gtfs::StopInfo;
// pub use http::{Request as HttpRequest, Response as HttpResponse};

/// The `Provider` trait is implemented by library users in order to provide
/// source data and caching services required by the application.
pub trait Provider: Source + Clone + Send + Sync {}

/// The `Source` trait defines the behavior for fetching data from a source.
#[async_trait]
pub trait Source: Send + Sync {
    async fn fetch(&self, owner: &str, key: &Key) -> Result<SourceData>;
}

/// Key for caching and fetching data.
#[derive(Debug, Clone)]
pub enum Key {
    /// Key to use for stop information.
    StopInfo(String),

    /// Key to use for Block Management data.
    BlockMgt(String),
}

/// Source data types.
#[derive(Debug, Clone)]
pub enum SourceData {
    /// GTFS stop information.
    StopInfo(StopInfo),

    /// Block management API data.
    BlockMgt(Vec<String>),
}
