use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{Mutex, OwnedMutexGuard};

// Replaces AsyncLock usage from legacy/at_smartrak_gtfs_adapter/src/processors/{location,serial-data}.ts.
#[derive(Debug, Clone, Default)]
pub struct KeyLocker {
    inner: Arc<KeyLockerInner>,
}

impl KeyLocker {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn lock(&self, key: impl Into<String>) -> KeyLockGuard {
        let key = key.into();
        let lock =
            self.inner.locks.entry(key.clone()).or_insert_with(|| Arc::new(Mutex::new(()))).clone();
        let guard = lock.lock_owned().await;
        KeyLockGuard { key, inner: Arc::clone(&self.inner), guard: Some(guard) }
    }
}

#[derive(Debug)]
struct KeyLockerInner {
    locks: DashMap<String, Arc<Mutex<()>>>,
}

impl Default for KeyLockerInner {
    fn default() -> Self {
        Self { locks: DashMap::new() }
    }
}

pub struct KeyLockGuard {
    key: String,
    inner: Arc<KeyLockerInner>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for KeyLockGuard {
    fn drop(&mut self) {
        self.guard.take();
        if let Some(entry) = self.inner.locks.get(&self.key) {
            if Arc::strong_count(entry.value()) == 1 {
                self.inner.locks.remove(&self.key);
            }
        }
    }
}
