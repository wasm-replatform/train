use anyhow::Context as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{
    Config, HttpRequest, Identity, Message, Publisher, Result, StateStore, bad_request,
};

use crate::location::Location;
use crate::{god_mode, location, serial_data};

async fn handle<P>(_owner: &str, message: SmarTrakMessage, provider: &P) -> Result<Reply<()>>
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    // serial data event
    if message.event_type == EventType::SerialData {
        let mut message = message.clone();
        if let Some(god_mode) = god_mode::god_mode() {
            god_mode.preprocess(&mut message);
        }
        serial_data::process(&message, provider).await?;
        return Ok(Reply::ok(()));
    }

    // must be a location event
    let Some(location) = location::process(&message, provider).await? else {
        return Ok(Reply::ok(()));
    };

    let (payload, key, topic) = match location {
        Location::VehiclePosition(feed) => {
            (serde_json::to_vec(&feed)?, feed.id, "realtime-gtfs-vp.v1")
        }
        Location::DeadReckoning(dr) => {
            (serde_json::to_vec(&dr)?, dr.id, "realtime-dead-reckoning.v1")
        }
    };

    // publish
    let mut message = Message::new(&payload);
    message.headers.insert("key".to_string(), key.clone());
    Publisher::send(provider, topic, &message).await?;

    Ok(Reply::ok(()))
}

impl<P> Handler<P> for SmarTrakMessage
where
    P: Config + HttpRequest + Identity + Publisher + StateStore,
{
    type Error = warp_sdk::Error;
    type Input = Vec<u8>;
    type Output = ();

    fn from_input(input: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&input).context("deserializing SmarTrakMessage").map_err(Into::into)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<()>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmarTrakMessage {
    #[serde(rename = "eventType")]
    pub event_type: EventType,
    pub remote_data: Option<RemoteData>,
    pub message_data: MessageData,
    #[serde(default)]
    pub location_data: LocationData,
    #[serde(default)]
    pub event_data: EventData,
    pub serial_data: Option<SerialData>,
}

impl SmarTrakMessage {
    pub(crate) fn timestamp(&self) -> Result<i64> {
        DateTime::parse_from_rfc3339(&self.message_data.timestamp)
            .map(|dt| dt.with_timezone(&Utc).timestamp())
            .map_err(|e| bad_request!("invalid timestamp: {}", e))
    }

    pub(crate) fn vehicle_id(&self) -> Option<&str> {
        self.remote_data
            .as_ref()
            .and_then(|rd| rd.external_id.as_deref().or(rd.remote_name.as_deref()))
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum EventType {
    #[serde(rename = "serialData", alias = "SERIAL_DATA")]
    SerialData,

    #[serde(rename = "location", alias = "LOCATION")]
    Location,

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteData {
    pub external_id: Option<String>,
    pub remote_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub heading: Option<f64>,
    pub speed: Option<f64>,
    pub odometer: Option<f64>,
    #[serde(default)]
    pub gps_accuracy: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    pub odometer: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialData {
    pub decoded_serial_data: Option<DecodedSerialData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedSerialData {
    #[serde(alias = "tripNumber")]
    pub trip_number: Option<String>,
    #[serde(alias = "tripId")]
    pub trip_id: Option<String>,
    #[serde(alias = "lineId")]
    pub line_id: Option<String>,
}
