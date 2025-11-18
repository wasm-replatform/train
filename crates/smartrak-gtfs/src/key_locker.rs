use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Default)]
pub struct KeyLocker {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    locks: DashMap<String, Arc<Mutex<()>>>,
}

impl KeyLocker {
    pub async fn lock(&self, key: &str) -> KeyGuard {
        let entry = self
            .inner
            .locks
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        let guard = entry.lock_owned().await;
        KeyGuard { key: key.to_string(), inner: Arc::clone(&self.inner), guard }
    }
}

pub struct KeyGuard {
    key: String,
    inner: Arc<Inner>,
    #[allow(dead_code)]
    guard: OwnedMutexGuard<()>,
}

impl Drop for KeyGuard {
    fn drop(&mut self) {
        if self
            .inner
            .locks
            .get(&self.key)
            .is_some_and(|existing| Arc::strong_count(existing.value()) == 1)
        {
            self.inner.locks.remove(&self.key);
        }
    }
}
