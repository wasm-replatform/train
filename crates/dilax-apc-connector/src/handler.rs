use anyhow::Context as _;
use fabric::api::{Context, Handler, Headers, Reply};
use fabric::{Error, IntoBody, Message, Publisher, Result};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};

use crate::DilaxMessage;

const DILAX_TOPIC: &str = "realtime-dilax-apc.v2";

#[allow(clippy::unused_async)]
async fn handle<P>(_owner: &str, request: DilaxRequest, provider: &P) -> Result<Reply<DilaxReply>>
where
    P: Publisher,
{
    let message = &request.message;

    // TODO: forward to replication topic/endpoint
    // if (Config.replication.endpoint) {
    //     this.eventStore.put(req.body);
    // }

    // forward to dilax-adapter topic
    let msg_vec = serde_json::to_vec(message).context("failed to serialize DilaxMessage")?;
    let mut msg = Message::new(&msg_vec);
    let site = message.device.as_ref().map_or_else(|| "undefined", |device| &device.site);
    msg.headers.insert("key".to_string(), site.to_string());
    Publisher::send(provider, DILAX_TOPIC, &msg).await?;

    Ok(Reply {
        status: StatusCode::OK,
        headers: HeaderMap::from_iter([(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        )]),
        body: DilaxReply("OK"),
    })
}

impl<P> Handler<P> for DilaxRequest
where
    P: Publisher,
{
    type Error = Error;
    type Output = DilaxReply;

    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<DilaxReply>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}

/// Dilax request
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct DilaxRequest {
    /// Dilax message
    pub message: DilaxMessage,
}

impl TryFrom<&[u8]> for DilaxRequest {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct DilaxReply(pub &'static str);

impl IntoBody for DilaxReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::types::DilaxMessage;

    #[tokio::test]
    async fn publishes_device_site_header() {
        let provider = MockProvider::default();
        let message = sample_message();
        let request = DilaxRequest { message: message.clone() };

        handle("owner", request, &provider).await.expect("handler should succeed");

        let published = provider.published();
        assert_eq!(published.len(), 1);

        let (topic, record) = &published[0];
        assert_eq!(topic, DILAX_TOPIC);
        let expected_key = message.device.as_ref().expect("device").site.as_str();
        assert_eq!(record.headers.get("key").map(String::as_str), Some(expected_key));
        assert_eq!(record.payload, serde_json::to_vec(&message).expect("serialize"));
    }

    #[tokio::test]
    async fn publishes_undefined_header_when_device_missing() {
        let provider = MockProvider::default();
        let mut message = sample_message();
        message.device = None;
        let request = DilaxRequest { message: message.clone() };

        handle("owner", request, &provider).await.expect("handler should succeed");

        let published = provider.published();
        assert_eq!(published.len(), 1);

        let (topic, record) = &published[0];
        assert_eq!(topic, DILAX_TOPIC);
        assert_eq!(record.headers.get("key").map(String::as_str), Some("undefined"));
        assert_eq!(record.payload, serde_json::to_vec(&message).expect("serialize"));
    }

    #[tokio::test]
    async fn publishes_whitespace_when_device_site_whitespace() {
        let provider = MockProvider::default();
        let site = "  ";
        let mut message = sample_message();
        if let Some(device) = message.device.as_mut() {
            device.site = site.to_string();
        }
        let request = DilaxRequest { message: message.clone() };

        handle("owner", request, &provider).await.expect("handler should succeed");

        let published = provider.published();
        assert_eq!(published.len(), 1);

        let (topic, record) = &published[0];
        assert_eq!(topic, DILAX_TOPIC);
        assert_eq!(record.headers.get("key").map(String::as_str), Some(site));
        assert_eq!(record.payload, serde_json::to_vec(&message).expect("serialize"));
    }

    #[derive(Default, Clone)]
    struct MockProvider {
        published: Arc<Mutex<Vec<(String, Message)>>>,
    }

    impl MockProvider {
        fn published(&self) -> Vec<(String, Message)> {
            self.published.lock().expect("lock").clone()
        }
    }

    impl Publisher for MockProvider {
        fn send(
            &self, topic: &str, message: &Message,
        ) -> impl Future<Output = anyhow::Result<()>> + Send {
            let topic = topic.to_string();
            let message = message.clone();
            let published = Arc::clone(&self.published);

            async move {
                published.lock().expect("lock").push((topic, message));
                Ok(())
            }
        }
    }

    fn sample_message() -> DilaxMessage {
        serde_json::from_str(include_str!("../data/dilax_sample.json"))
            .expect("fixture should deserialize")
    }
}
