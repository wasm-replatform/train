use std::any::Any;
use std::env;
use std::error::Error;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use bytes::Bytes;
use dilax::{HttpRequest as DilaxHttpRequest, StateStore};
use http::{Request, Response};
use r9k_position::{HttpRequest as R9kHttpRequest, Identity};
use tracing::warn;
use wasi_identity::credentials::get_identity;
use wasi_keyvalue::{self, TtlValue, store};
use wit_bindgen::block_on;

#[derive(Clone, Default)]
pub struct Provider;

impl r9k_position::Provider for Provider {}

impl R9kHttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}

impl dilax::Provider for Provider {}

impl DilaxHttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}

impl StateStore for Provider {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let bucket = state_bucket()?;
        load_value(bucket, key)
    }

    async fn set(
        &self, key: &str, value: &[u8], expires: Option<chrono::Duration>,
    ) -> Result<Option<Vec<u8>>> {
        let bucket = state_bucket()?;
        let previous = load_value(bucket, key)?;
        let ttl_seconds = ttl_seconds(expires)?;
        let payload = encode_value(value, ttl_seconds)?;

        bucket
            .set(key, &payload)
            .map_err(|err| anyhow::anyhow!("setting state entry `{key}`: {err}"))?;

        Ok(previous)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let bucket = state_bucket()?;
        bucket.delete(key).map_err(|err| anyhow::anyhow!("deleting state entry `{key}`: {err}"))?;
        Ok(())
    }
}

impl Identity for Provider {
    async fn access_token(&self) -> Result<String> {
        let identity = env::var("AZURE_IDENTITY")?;
        let identity = block_on(get_identity(identity))?;
        let access_token = block_on(async move { identity.get_token(vec![]).await })?;
        Ok(access_token.token)
    }
}

fn state_bucket() -> Result<&'static wasi_keyvalue::store::Bucket> {
    static BUCKET: OnceLock<wasi_keyvalue::store::Bucket> = OnceLock::new();
    if let Some(bucket) = BUCKET.get() {
        return Ok(bucket);
    }

    let bucket = store::open("train_bucket").context("opening bucket")?;
    Ok(BUCKET.get_or_init(move || bucket))
}

fn load_value(bucket: &wasi_keyvalue::store::Bucket, key: &str) -> Result<Option<Vec<u8>>> {
    let maybe =
        bucket.get(key).map_err(|err| anyhow::anyhow!("reading state entry `{key}`: {err}"))?;

    let Some(bytes) = maybe else {
        return Ok(None);
    };

    match serde_json::from_slice::<TtlValue>(&bytes) {
        Ok(envelope) => {
            if let (Some(ttl), Some(timestamp)) = (envelope.ttl_seconds, envelope.timestamp_seconds)
                && ttl > 0
            {
                let now = current_timestamp_seconds()?;
                if now >= timestamp.saturating_add(ttl) {
                    if let Err(err) = bucket.delete(key) {
                        warn!(key = %key, error = %err, "failed to delete expired state entry");
                    }
                    return Ok(None);
                }
            }

            Ok(Some(envelope.value))
        }
        Err(_) => Ok(Some(bytes)),
    }
}

fn ttl_seconds(expires: Option<chrono::Duration>) -> Result<Option<u64>> {
    let Some(duration) = expires else {
        return Ok(None);
    };

    let seconds = duration.num_seconds();
    if seconds <= 0 {
        return Ok(None);
    }

    let ttl = u64::try_from(seconds).context("ttl exceeds u64 range")?;
    Ok(Some(ttl))
}

fn encode_value(value: &[u8], ttl_seconds: Option<u64>) -> Result<Vec<u8>> {
    let Some(ttl) = ttl_seconds else {
        return Ok(value.to_vec());
    };

    let envelope = TtlValue {
        value: value.to_vec(),
        ttl_seconds: Some(ttl),
        timestamp_seconds: Some(current_timestamp_seconds()?),
    };

    serde_json::to_vec(&envelope).context("serializing state entry envelope")
}

fn current_timestamp_seconds() -> Result<u64> {
    let timestamp = chrono::Utc::now().timestamp();
    u64::try_from(timestamp).context("timestamp precedes unix epoch")
}
