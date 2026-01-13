use anyhow::Context as _;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use qwasr_sdk::{Config, Context, Error, Handler, IntoBody, Message, Publisher, Reply, Result};
use serde::{Deserialize, Serialize};

use crate::DilaxMessage;

const DILAX_TOPIC: &str = "realtime-dilax-apc.v2";

#[allow(clippy::unused_async)]
async fn handle<P>(_owner: &str, request: DilaxRequest, provider: &P) -> Result<Reply<DilaxReply>>
where
    P: Config + Publisher,
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

    let env = Config::get(provider, "ENV").await.unwrap_or_else(|_| "dev".to_string());
    let topic = format!("{env}-{DILAX_TOPIC}");

    Publisher::send(provider, &topic, &msg).await?;

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
    P: Config + Publisher,
{
    type Error = Error;
    type Input = Vec<u8>;
    type Output = DilaxReply;

    fn from_input(input: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&input).map_err(Into::into)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<DilaxReply>> {
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

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct DilaxReply(pub &'static str);

impl IntoBody for DilaxReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.as_bytes().to_vec())
    }
}
