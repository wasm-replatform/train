//! R9K HTTP Connector
//!
//! Listen for incoming R9K SOAP requests and forward to the r9k-adapter topic
//! for validation and transformation to SmarTrak events.

use std::fmt::{self, Display};
use std::str::FromStr;

use credibil_api::{Handler, Request, Response};
use serde::{Deserialize, Serialize};

use crate::{Error, Message, Provider, Publisher, Result};

const R9K_TOPIC: &str = "realtime-r9k.v1";
const ERROR: Fault =
    Fault { status_code: 500, response: FaultMessage { message: "Internal Server Error" } };

#[allow(clippy::unused_async)]
async fn handle(
    _owner: &str, request: R9kRequest, provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    let message = &request.body.receive_message.axml_message;

    // verify message
    if message.is_empty() || !message.contains("<ActualizarDatosTren>") {
        return Err(Error::Unprocessable(ERROR.to_string()));
    }

    // TODO: forward to replication topic/endpoint
    // if (Config.replication.endpoint) {
    //     this.eventStore.put(req.body);
    // }

    // forward to r9k-adapter topic
    let msg = Message::new(message.as_bytes());
    Publisher::send(provider, R9K_TOPIC, &msg).await?;

    Ok(R9kResponse("OK").into())
}

impl<P: Provider> Handler<R9kResponse, P> for Request<R9kRequest> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<R9kResponse>> {
        handle(owner, self.body, provider).await
    }
}

/// R9K SOAP Envelope for incoming [`ReceiveMessage`] requests
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct R9kRequest {
    /// SOAP Body
    pub body: Body,
}

impl FromStr for R9kRequest {
    type Err = Error;

    fn from_str(xml: &str) -> anyhow::Result<Self, Self::Err> {
        quick_xml::de::from_str(xml).map_err(Into::into)
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
pub struct R9kResponse(pub &'static str);

impl Display for R9kResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let xml = quick_xml::se::to_string(&self).map_err(|_e| fmt::Error)?;
        write!(f, "{xml}",)
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
    use std::str::FromStr;

    use super::*;

    #[test]
    fn deserialize_soap() {
        let xml = include_str!("../data/receive-message.xml");
        let envelope = R9kRequest::from_str(xml).expect("should deserialize");

        let receive_message = envelope.body.receive_message;
        let message = receive_message.axml_message;

        assert!(!message.is_empty());
        assert!(message.contains("<ActualizarDatosTren>"));
    }

    #[test]
    fn serialize_ok() {
        let xml = R9kResponse("OK").to_string();
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
