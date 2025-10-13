//! # Provider
//!
//! The `Provider` module defines the traits required to integrate
//! external (infrastructural) services into the application.

use anyhow::Result;

use crate::gtfs::StopInfo;
// pub use http::{Request as HttpRequest, Response as HttpResponse};

/// The `Provider` trait is implemented by library users in order to provide
/// source data and caching services required by the application.
pub trait Provider: Source + Clone + Send + Sync {}

/// The `Source` trait defines the behavior for fetching data from a source.
pub trait Source: Send + Sync {
    /// Fetches data from the source using the provided key. The return type
    /// is strongly typed to match the expected source data structure.
    fn fetch(&self, owner: &str, key: &Key) -> impl Future<Output = Result<SourceData>> + Send;
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
