#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_pass_by_value)]

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use anyhow::{Context, Result, anyhow};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use wasi_keyvalue::store;

    #[derive(Clone)]
    pub struct KvStore {
        bucket: Arc<store::Bucket>,
    }

    #[derive(Serialize, Deserialize)]
    struct TtlEnvelope {
        expires_at: i64,
        value: Vec<u8>,
    }

    #[derive(Default, Serialize, Deserialize)]
    struct SetEnvelope {
        expires_at: Option<i64>,
        members: Vec<String>,
    }

    impl KvStore {
        pub fn open(name: &str) -> Result<Self> {
            let bucket =
                store::open(name).map_err(|err| anyhow!("failed to open bucket {name}: {err}"))?;
            Ok(Self { bucket: Arc::new(bucket) })
        }

        pub fn get_string(&self, key: &str) -> Result<Option<String>> {
            match self.bucket.get(key).map_err(map_store_err)? {
                Some(raw) => {
                    let value = String::from_utf8(raw)
                        .with_context(|| format!("value for key {key} was not valid UTF-8"))?;
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        }

        pub fn set_string(&self, key: &str, value: &str) -> Result<()> {
            self.bucket.set(key, value.as_bytes()).map_err(map_store_err)
        }

        pub fn get_with_ttl(&self, key: &str) -> Result<Option<Vec<u8>>> {
            let Some(raw) = self.bucket.get(key).map_err(map_store_err)? else {
                return Ok(None);
            };

            match serde_json::from_slice::<TtlEnvelope>(&raw) {
                Ok(envelope) => {
                    if envelope.expires_at <= now_unix_timestamp() {
                        self.bucket.delete(key).map_err(map_store_err)?;
                        Ok(None)
                    } else {
                        Ok(Some(envelope.value))
                    }
                }
                Err(_) => Ok(Some(raw)),
            }
        }

        pub fn replace_with_ttl(
            &self, key: &str, value: &[u8], ttl: Duration,
        ) -> Result<Option<Vec<u8>>> {
            let previous = self.get_with_ttl(key)?;
            self.store_with_ttl(key, value, ttl)?;
            Ok(previous)
        }

        pub fn set_string_with_ttl(&self, key: &str, value: &str, ttl: Duration) -> Result<()> {
            self.store_with_ttl(key, value.as_bytes(), ttl)
        }

        pub fn set_json_with_ttl<T: Serialize>(
            &self, key: &str, value: &T, ttl: Duration,
        ) -> Result<()> {
            let bytes = serde_json::to_vec(value)?;
            self.store_with_ttl(key, &bytes, ttl)
        }

        pub fn get_json_with_ttl<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
            self.get_with_ttl(key)?
                .map(|raw| {
                    serde_json::from_slice(&raw).with_context(|| {
                        format!("failed to deserialize payload stored at key {key}")
                    })
                })
                .transpose()
        }

        pub fn set_members(&self, key: &str) -> Result<Vec<String>> {
            Ok(self.load_set(key)?.members)
        }

        pub fn add_to_set(&self, key: &str, member: &str) -> Result<()> {
            let mut set = self.load_set(key)?;
            if !set.members.iter().any(|existing| existing == member) {
                set.members.push(member.to_string());
                self.store_set(key, &set)?;
            }
            Ok(())
        }

        pub fn set_expiry(&self, key: &str, ttl: Duration) -> Result<()> {
            let mut set = self.load_set(key)?;
            set.expires_at = Some(deadline(ttl));
            self.store_set(key, &set)
        }

        fn store_with_ttl(&self, key: &str, value: &[u8], ttl: Duration) -> Result<()> {
            let envelope = TtlEnvelope { expires_at: deadline(ttl), value: value.to_vec() };
            let bytes = serde_json::to_vec(&envelope)?;
            self.bucket.set(key, &bytes).map_err(map_store_err)
        }

        fn load_set(&self, key: &str) -> Result<SetEnvelope> {
            let Some(raw) = self.bucket.get(key).map_err(map_store_err)? else {
                return Ok(SetEnvelope::default());
            };

            match serde_json::from_slice::<SetEnvelope>(&raw) {
                Ok(set) => {
                    if set.expires_at.is_some_and(|expires_at| expires_at <= now_unix_timestamp()) {
                        self.bucket.delete(key).map_err(map_store_err)?;
                        Ok(SetEnvelope::default())
                    } else {
                        Ok(set)
                    }
                }
                Err(_) => Ok(SetEnvelope::default()),
            }
        }

        fn store_set(&self, key: &str, set: &SetEnvelope) -> Result<()> {
            let bytes = serde_json::to_vec(set)?;
            self.bucket.set(key, &bytes).map_err(map_store_err)
        }
    }

    fn deadline(ttl: Duration) -> i64 {
        let now = now_unix_timestamp();
        let delta = i64::try_from(ttl.as_secs()).unwrap_or(i64::MAX);
        now.saturating_add(delta)
    }

    fn now_unix_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .try_into()
            .unwrap_or(i64::MAX)
    }

    fn map_store_err(err: store::Error) -> anyhow::Error {
        anyhow!("keyvalue store operation failed: {err}")
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::KvStore;

#[cfg(not(target_arch = "wasm32"))]
mod host_stub {
    use std::time::Duration;

    use anyhow::{Result, bail};
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    #[derive(Clone, Default)]
    pub struct KvStore;

    impl KvStore {
        pub fn open(_name: &str) -> Result<Self> {
            bail!("KvStore is only available when targeting wasm32");
        }

        pub fn get_string(&self, _key: &str) -> Result<Option<String>> {
            bail!("KvStore::get_string requires wasm32 target");
        }

        pub fn set_string(&self, _key: &str, _value: &str) -> Result<()> {
            bail!("KvStore::set_string requires wasm32 target");
        }

        pub fn get_with_ttl(&self, _key: &str) -> Result<Option<Vec<u8>>> {
            bail!("KvStore::get_with_ttl requires wasm32 target");
        }

        pub fn replace_with_ttl(
            &self, _key: &str, _value: &[u8], _ttl: Duration,
        ) -> Result<Option<Vec<u8>>> {
            bail!("KvStore::replace_with_ttl requires wasm32 target");
        }

        pub fn set_string_with_ttl(&self, _key: &str, _value: &str, _ttl: Duration) -> Result<()> {
            bail!("KvStore::set_string_with_ttl requires wasm32 target");
        }

        pub fn set_json_with_ttl<T: Serialize>(
            &self, _key: &str, _value: &T, _ttl: Duration,
        ) -> Result<()> {
            bail!("KvStore::set_json_with_ttl requires wasm32 target");
        }

        pub fn get_json_with_ttl<T: DeserializeOwned>(&self, _key: &str) -> Result<Option<T>> {
            bail!("KvStore::get_json_with_ttl requires wasm32 target");
        }

        pub fn set_members(&self, _key: &str) -> Result<Vec<String>> {
            bail!("KvStore::set_members requires wasm32 target");
        }

        pub fn add_to_set(&self, _key: &str, _member: &str) -> Result<()> {
            bail!("KvStore::add_to_set requires wasm32 target");
        }

        pub fn set_expiry(&self, _key: &str, _ttl: Duration) -> Result<()> {
            bail!("KvStore::set_expiry requires wasm32 target");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use host_stub::KvStore;

// The watcher export is registered from the root crate.
