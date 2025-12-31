//! R9K HTTP Connector
//!
//! Listen for incoming R9K SOAP requests and forward to the r9k-adapter topic
//! for validation and transformation to SmarTrak events.

use std::fmt::{self, Display};

use anyhow::Context as _;
use fabric::api::{Context, Handler, Headers, Reply};
use fabric::{Error, IntoBody, Message, Publisher, Result, bad_request};
use serde::{Deserialize, Serialize};

use crate::R9kError;

const R9K_TOPIC: &str = "realtime-r9k.v1";
const ERROR: Fault =
    Fault { status_code: 500, response: FaultMessage { message: "Internal Server Error" } };

#[allow(clippy::unused_async)]
async fn handle<P>(_owner: &str, request: R9kRequest, provider: &P) -> Result<Reply<R9kReply>>
where
    P: Publisher,
{
    let message = &request.body.receive_message.axml_message;

    // verify message
    if message.is_empty() || !message.contains("<ActualizarDatosTren>") {
        return Err(bad_request!("{ERROR}"));
    }

    // TODO: forward to replication topic/endpoint
    // if (Config.replication.endpoint) {
    //     this.eventStore.put(req.body);
    // }

    // forward to r9k-adapter topic
    let msg = Message::new(message.as_bytes());
    Publisher::send(provider, R9K_TOPIC, &msg).await?;

    Ok(R9kReply("OK").into())
}

impl<P> Handler<P> for R9kRequest
where
    P: Publisher,
{
    type Error = Error;
    type Output = R9kReply;

    // TODO: implement "owner"
    async fn handle<H>(self, ctx: Context<'_, P, H>) -> Result<Reply<R9kReply>>
    where
        H: Headers,
    {
        handle(ctx.owner, self, ctx.provider).await
    }
}

/// R9K SOAP Envelope for incoming [`ReceiveMessage`] requests
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct R9kRequest {
    /// SOAP Body
    pub body: Body,
}

impl TryFrom<&[u8]> for R9kRequest {
    type Error = R9kError;

    fn try_from(value: &[u8]) -> anyhow::Result<Self, Self::Error> {
        quick_xml::de::from_reader(value).map_err(Into::into)
    }
}

/// R9K SOAP Body for [`ReceiveMessage`] requests
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Body {
    pub receive_message: ReceiveMessage,
}

/// R9K SOAP wrapper for train position messages.
#[derive(Debug, Clone, Deserialize)]
pub struct ReceiveMessage {
    #[serde(rename = "AXMLMessage")]
    pub axml_message: String,
}

/// R9K SOAP Response
#[derive(Debug, Clone, Serialize)]
#[serde(rename = "Return")]
pub struct R9kReply(pub &'static str);

impl IntoBody for R9kReply {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        let xml = quick_xml::se::to_string(&self).context("serializing R9kResponse")?;
        Ok(xml.into_bytes())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Fault {
    status_code: u16,
    response: FaultMessage,
}

impl Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let xml = quick_xml::se::to_string(&self).map_err(|_e| fmt::Error)?;
        write!(f, "{xml}",)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FaultMessage {
    pub message: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_soap() {
        let xml = include_str!("../data/receive-message.xml");
        let envelope = R9kRequest::try_from(xml.as_bytes()).expect("should deserialize");

        let receive_message = envelope.body.receive_message;
        let message = receive_message.axml_message;

        assert!(!message.is_empty());
        assert!(message.contains("<ActualizarDatosTren>"));
    }

    #[test]
    fn serialize_ok() {
        let xml = R9kReply("OK").into_body().expect("should serialize");
        let xml = String::from_utf8(xml).expect("should be UTF-8");
        assert_eq!(xml, "<Return>OK</Return>");
    }

    #[test]
    fn serialize_error() {
        let xml = ERROR.to_string();
        assert_eq!(
            xml,
            "<Fault><StatusCode>500</StatusCode><Response><Message>Internal Server Error</Message></Response></Fault>"
        );
    }
}
