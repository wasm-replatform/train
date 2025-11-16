//! R9K HTTP Connector
//!
//! Listen for incoming R9K SOAP requests and forward to the r9k-adapter topic
//! for validation and transformation to SmarTrak events.

use std::fmt::{self, Display};
use std::str::FromStr;

use credibil_api::{Handler, Request, Response};
use serde::Deserialize;
use serde::Serialize;

use crate::provider::Provider;
use crate::{Error, Result};

const OK: R9kResponse = R9kResponse::Ok { r#return: "Internal Server Error" };
const ERROR: R9kResponse = R9kResponse::Fault {
    status_code: 500,
    response: FaultMessage { message: "Internal Server Error" },
};

#[allow(clippy::unused_async)]
async fn handle(
    _owner: &str, request: R9kRequest, _provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    let xml: R9kResponse = match process_message(request) {
        Ok(()) => OK,
        Err(_e) => ERROR,
    };

    Ok(xml.into())
}

#[allow(clippy::unnecessary_wraps)]
fn process_message(_request: R9kRequest) -> Result<()> {
    Ok(())
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
pub struct R9kRequest {
    /// SOAP Body
    #[serde(rename = "Body")]
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
pub struct Body {
    #[serde(rename = "ReceiveMessage")]
    pub receive_message: ReceiveMessage,
}

/// R9K SOAP wrapper for train position messages.
#[derive(Debug, Clone, Deserialize)]
pub struct ReceiveMessage {
    #[serde(rename = "AXMLMessage")]
    pub message: String,
}

/// R9K SOAP Response
#[derive(Debug, Clone, Serialize)]
pub enum R9kResponse {
    Ok { r#return: &'static str },
    Fault { status_code: u16, response: FaultMessage },
}

impl Display for R9kResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let xml = quick_xml::se::to_string(&self).map_err(|_e| fmt::Error)?;
        write!(f, "{xml}",)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FaultMessage {
    pub message: &'static str,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::R9kRequest;

    #[test]
    fn deserialize_soap() {
        let xml = include_str!("../data/soap.xml");

        let envelope = R9kRequest::from_str(xml).expect("should deserialize");

        let receive_message = envelope.body.receive_message;
        let message = receive_message.message;

        assert!(!message.is_empty());
        assert!(message.contains("<ActualizarDatosTren>"));
    }
}
