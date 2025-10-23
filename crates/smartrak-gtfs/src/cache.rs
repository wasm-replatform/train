use std::fmt;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;
use wit_bindings::keyvalue::store;
use wit_bindings::keyvalue::store::Bucket;

const EMPTY_SENTINEL: &str = "__empty__";

#[async_trait]
pub trait CacheStore: Send + Sync + Clone + 'static {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set_ex(&self, key: &str, ttl: Duration, value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

pub struct CacheRepository {
    store: Bucket,
}

impl CacheRepository {
    // Mirrors legacy cache repository at legacy/at_smartrak_gtfs_adapter/src/repositories/cache.ts.
    pub fn new() -> Result<Self> {
        let bucket = store::open("smartrak").context("opening bucket")?;
        Ok(Self { store: bucket })
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let Some(bytes) = self.store.get(key).context("getting key from bucket")? else {
            return Ok(None);
        };
        if bytes == EMPTY_SENTINEL.as_bytes() {
            return Ok(None);
        }
        match String::from_utf8(bytes) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                warn!(key = key, error = %err, "failed to decode cached UTF-8 value");
                let _ = self.store.delete(key);
                Ok(None)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn get_json<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let Some(bytes) = self.store.get(key).context("getting key from bucket")? else {
            return Ok(None);
        };
        if bytes == EMPTY_SENTINEL.as_bytes() {
            return Ok(None);
        }

        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                warn!(key = key, error = %err, "failed to deserialize cached JSON value");
                let _ = self.store.delete(key);
                Ok(None)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn set_ex(&self, key: &str, _ttl: Duration, value: impl Into<String>) -> Result<()> {
        let payload = value.into().into_bytes();
        self.store.set(key, &payload).context("setting value")
        //self.store.set_with_ttl(key, payload, ttl)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn set_json_ex<T>(&self, key: &str, _ttl: Duration, value: &T) -> Result<()>
    where
        T: Serialize + Sync,
    {
        let payload = serde_json::to_vec(value)?;
        self.store.set(key, &payload).context("setting value")
        //self.store.set_with_ttl(key, payload, ttl).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn set_empty(&self, key: &str, _ttl: Duration) -> Result<()> {
        self.store.set(key, EMPTY_SENTINEL.as_bytes()).context("setting value")
        //self.store.set_with_ttl(key, EMPTY_SENTINEL.as_bytes().to_vec(), ttl).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn delete(&self, key: &str) -> Result<()> {
        self.store.delete(key).context("Deleting key")
    }
}

impl fmt::Debug for CacheRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheRepository").finish()
    }
}
