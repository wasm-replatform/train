use std::fmt;
use std::time::Duration;

use anyhow::{ Result, Context };
use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;
use wit_bindings::keyvalue::store;

const EMPTY_SENTINEL: &str = "__empty__";

#[async_trait]
pub trait CacheStore: Send + Sync + Clone + 'static {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set_ex(&self, key: &str, ttl: Duration, value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

impl<C: CacheStore> CacheRepository<C> {
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let bucket = store::open("tomtom").context("opening bucket")?;
        let Some(bytes) = bucket.get(key).context("getting key from bucket")? else {
            return Ok(None);
        };
        if bytes == EMPTY_SENTINEL.as_bytes() {
            return Ok(None);
        }
        match String::from_utf8(bytes) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                warn!(key = key, error = %err, "failed to decode cached UTF-8 value");
                let _ = bucket.delete(key);
                Ok(None)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_json<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
         let bucket = store::open("tomtom").context("opening bucket")?;
        let Some(bytes) = bucket.get(key).context("getting key from bucket")? else {
            return Ok(None);
        };
        if bytes == EMPTY_SENTINEL.as_bytes() {
            return Ok(None);
        }

        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                warn!(key = key, error = %err, "failed to deserialize cached JSON value");
                let _ = bucket.delete(key);
                Ok(None)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn set_ex(&self, key: &str, ttl: Duration, value: impl Into<String>) -> Result<()> {
        let bucket = store::open("tomtom").context("opening bucket")?;
        let payload = value.into().into_bytes();
        bucket.set_with_ttl(key, payload, ttl)
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn set_json_ex<T>(&self, key: &str, ttl: Duration, value: &T) -> Result<()>
    where
        T: Serialize + Sync,
    {
        let bucket = store::open("tomtom").context("opening bucket")?;
        let payload = serde_json::to_vec(value)?;
        bucket.set_with_ttl(key, payload, ttl).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn set_empty(&self, key: &str, ttl: Duration) -> Result<()> {
        let bucket = store::open("tomtom").context("opening bucket")?;
        bucket.set_with_ttl(key, EMPTY_SENTINEL.as_bytes().to_vec(), ttl).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn delete(&self, key: &str) -> Result<()> {
        let bucket = store::open("tomtom").context("opening bucket")?;
        bucket.delete(key)
    }
}

impl<C: CacheStore> fmt::Debug for CacheRepository<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheRepository").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheRepository, CacheStore};
    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    #[derive(Clone, Default)]
    struct MockStore {
        inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    #[async_trait::async_trait]
    impl CacheStore for MockStore {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            let map = self.inner.lock().await;
            Ok(map.get(key).cloned())
        }

        async fn set_ex(&self, key: &str, _ttl: Duration, value: Vec<u8>) -> Result<()> {
            let mut map = self.inner.lock().await;
            map.insert(key.to_string(), value);
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<()> {
            let mut map = self.inner.lock().await;
            map.remove(key);
            Ok(())
        }
    }

    impl MockStore {
        async fn set_raw(&self, key: &str, value: &[u8]) {
            let mut map = self.inner.lock().await;
            map.insert(key.to_string(), value.to_vec());
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        value: u32,
    }

    #[tokio::test]
    async fn set_json_round_trips() -> Result<()> {
        let store = MockStore::default();
        let repo = CacheRepository::new(store.clone());
        let payload = Payload { value: 42 };

        repo.set_json_ex("payload", Duration::from_secs(5), &payload).await?;
        let stored = repo.get_json::<Payload>("payload").await?;

        assert_eq!(stored, Some(payload));
        Ok(())
    }

    #[tokio::test]
    async fn empty_marker_behaves_like_none() -> Result<()> {
        let store = MockStore::default();
        let repo = CacheRepository::new(store.clone());

        repo.set_empty("empty", Duration::from_secs(5)).await?;

        assert!(repo.get_json::<Payload>("empty").await?.is_none());
        assert!(repo.get("empty").await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn invalid_json_is_evicted() -> Result<()> {
        let store = MockStore::default();
        let repo = CacheRepository::new(store.clone());

        store.set_raw("broken", b"not-json").await;

        assert!(repo.get_json::<Payload>("broken").await?.is_none());
        assert!(CacheStore::get(&store, "broken").await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn raw_utf8_errors_are_ignored() -> Result<()> {
        let store = MockStore::default();
        let repo = CacheRepository::new(store.clone());

        store.set_raw("utf8", &[0, 159, 146, 150]).await;

        assert!(repo.get("utf8").await?.is_none());
        assert!(CacheStore::get(&store, "utf8").await?.is_none());
        Ok(())
    }
}
