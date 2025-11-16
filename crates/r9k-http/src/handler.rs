//! R9K HTTP Connector
//!
//! Listen for incoming R9K SOAP requests and forward to the r9k-adapter topic
//! for validation and transformation to SmarTrak events.

use std::fmt::{self, Display};

use credibil_api::{Handler, Request, Response};
use serde::Serialize;

use crate::provider::Provider;
use crate::r9k::R9kMessage;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub struct FaultMessage {
    message: &'static str,
}

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

const OK: R9kResponse = R9kResponse::Ok { r#return: "Internal Server Error" };
const ERROR: R9kResponse = R9kResponse::Fault {
    status_code: 500,
    response: FaultMessage { message: "Internal Server Error" },
};

#[allow(clippy::unused_async)]
async fn handle(
    _owner: &str, request: R9kMessage, _provider: &impl Provider,
) -> Result<Response<R9kResponse>> {
    let xml: R9kResponse = match process_message(request) {
        Ok(()) => OK,
        Err(_e) => ERROR,
    };

    Ok(xml.into())
}

#[allow(clippy::unnecessary_wraps)]
fn process_message(_request: R9kMessage) -> Result<()> {
    Ok(())
}

impl<P: Provider> Handler<R9kResponse, P> for Request<R9kMessage> {
    type Error = Error;

    // TODO: implement "owner"
    async fn handle(self, owner: &str, provider: &P) -> Result<Response<R9kResponse>> {
        handle(owner, self.body, provider).await
    }
}
