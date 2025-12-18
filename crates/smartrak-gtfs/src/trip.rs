use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{Duration, NaiveDate, TimeZone, Timelike};
use chrono_tz::Tz;
use fabric::{Config, HttpRequest, Identity, Publisher, StateStore};
use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::{Method, StatusCode};
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

const CACHE_DIRECTIVE_PRIMARY: &str = "max-age=20, stale-if-error=10";

/// Retrieves the trip instance that matches the exact `trip_id`, `service_date`, and
/// `start_time` combination.
///
/// # Errors
///
/// Returns an error when the Trip Management API request fails or the response payload cannot
/// be deserialized.
pub async fn get_instance<P>(
    trip_id: &str, service_date: &str, start_time: &str, provider: &P,
) -> Result<Option<TripInstance>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let trips = fetch(trip_id, service_date, provider).await?;
    let mut iter = trips.into_iter();

    if let Some(first) = iter.next() {
        if first.has_error() {
            return Ok(Some(first));
        }

        if first.start_time == start_time {
            return Ok(Some(first));
        }

        for trip in iter {
            if trip.start_time == start_time {
                return Ok(Some(trip));
            }
        }
    }

    Ok(None)
}

/// Retrieves the closest trip instance to the supplied `event_timestamp`.
///
/// # Errors
///
/// Returns an error when Trip Management lookups fail or the payload cannot be decoded.
pub async fn get_nearest<P>(
    trip_id: &str, event_timestamp: i64, provider: &P,
) -> Result<Option<TripInstance>>
where
    P: HttpRequest + Publisher + StateStore + Identity + Config,
{
    let tz = chrono_tz::Pacific::Auckland;
    let Some(event_dt) = tz.timestamp_opt(event_timestamp, 0).single() else {
        return Ok(None);
    };

    let current_date = event_dt.format("%Y%m%d").to_string();
    let mut trips = fetch(trip_id, &current_date, provider).await?;

    if trips.first().is_some_and(TripInstance::has_error) {
        return Ok(trips.into_iter().next());
    }

    if event_dt.hour() < 4 {
        let previous_date = (event_dt - Duration::days(1)).format("%Y%m%d").to_string();
        let previous = fetch(trip_id, &previous_date, provider).await?;
        if previous.first().is_some_and(TripInstance::has_error) {
            return Ok(previous.into_iter().next());
        }
        trips.extend(previous);
    }

    if trips.is_empty() {
        return Ok(None);
    }

    trips.sort_by(|left, right| {
        let event_ts = event_dt.timestamp();
        let left_diff = difference(event_ts, left, tz);
        let right_diff = difference(event_ts, right, tz);
        left_diff.cmp(&right_diff)
    });

    Ok(trips.into_iter().next())
}

async fn fetch(
    trip_id: &str, service_date: &str, http: &impl HttpRequest,
) -> Result<Vec<TripInstance>> {
    let base_url = env::var("TRIP_MANAGEMENT_URL").context("getting `TRIP_MANAGEMENT_URL`")?;
    let endpoint = format!("{}/tripinstances", base_url.trim_end_matches('/'));

    let payload = serde_json::json!({
        "tripIds": [trip_id],
        "serviceDate": service_date,
    });
    let body_bytes = serde_json::to_vec(&payload).context("serializing trip management payload")?;

    let request = http::Request::builder()
        .method(Method::POST)
        .uri(&endpoint)
        .header(CACHE_CONTROL, CACHE_DIRECTIVE_PRIMARY)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body_bytes)))
        .context("building Trip Management request")?;

    let response = http.fetch(request).await.context("requesting trip instances")?;

    let status = response.status();
    let body = response.into_body();

    if status == StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !status.is_success() {
        warn!(%status, trip_id, service_date, "Trip Management API request failed");
        return Ok(vec![error_trip(service_date)]);
    }

    decode(&body)
        .with_context(|| format!("deserializing trip instances for {trip_id} on {service_date}"))
}

fn decode(payload: &[u8]) -> Result<Vec<TripInstance>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let value: Value = serde_json::from_slice(payload).context("parsing trip payload")?;
    extract(value)
}

fn extract(value: Value) -> Result<Vec<TripInstance>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => {
            let mut trips = Vec::new();
            for item in items {
                if matches!(&item, Value::Null)
                    || matches!(&item, Value::Object(map) if map.is_empty())
                {
                    continue;
                }
                let trip: TripInstance = serde_json::from_value(item)?;
                trips.push(trip);
            }
            Ok(trips)
        }
        Value::Object(mut map) => {
            if let Some(data) = map.remove("tripInstances") {
                return extract(data);
            }

            if let Some(data) = map.remove("data") {
                return extract(data);
            }

            if map.is_empty() {
                return Ok(Vec::new());
            }

            let trip: TripInstance = serde_json::from_value(Value::Object(map))?;
            Ok(vec![trip])
        }
        other => {
            let trip: TripInstance = serde_json::from_value(other)?;
            Ok(vec![trip])
        }
    }
}

fn difference(event_ts: i64, trip: &TripInstance, tz: Tz) -> i64 {
    let trip_ts = timestamp(trip, tz).unwrap_or(event_ts);
    (event_ts - trip_ts).abs()
}

fn timestamp(trip: &TripInstance, tz: Tz) -> Option<i64> {
    let date = NaiveDate::parse_from_str(&trip.service_date, "%Y%m%d").ok()?;
    let total_seconds = parse_time(&trip.start_time)?;
    let days = total_seconds.div_euclid(86_400);
    let remaining = total_seconds.rem_euclid(86_400);

    let hours = u32::try_from(remaining / 3_600).ok()?;
    let minutes = u32::try_from((remaining % 3_600) / 60).ok()?;
    let seconds = u32::try_from(remaining % 60).ok()?;

    let date = date + Duration::days(days);
    let local = date.and_hms_opt(hours, minutes, seconds)?;
    tz.from_local_datetime(&local).single().map(|dt| dt.timestamp())
}

fn parse_time(time: &str) -> Option<i64> {
    let mut parts = time.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    Some(hours * 3_600 + minutes * 60 + seconds)
}

fn error_trip(service_date: &str) -> TripInstance {
    TripInstance { service_date: service_date.to_string(), error: true, ..TripInstance::default() }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TripInstance {
    pub trip_id: String,
    pub route_id: String,
    pub service_date: String,
    pub start_time: String,
    pub end_time: String,
    pub direction_id: Option<i32>,
    pub is_added_trip: bool,
    #[serde(default)]
    pub error: bool,
}

impl TripInstance {
    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error
    }

    #[must_use]
    pub fn remap(&self, trip_id: &str, route_id: &str) -> Self {
        let mut clone = self.clone();
        clone.trip_id = trip_id.to_string();
        clone.route_id = route_id.to_string();
        clone
    }
}

impl From<&TripInstance> for TripDescriptor {
    fn from(inst: &TripInstance) -> Self {
        Self {
            trip_id: inst.trip_id.clone(),
            route_id: inst.route_id.clone(),
            start_time: Some(inst.start_time.clone()),
            start_date: Some(inst.service_date.clone()),
            direction_id: inst.direction_id,
            schedule_relationship: Some(if inst.is_added_trip {
                Self::ADDED.to_string()
            } else {
                Self::SCHEDULED.to_string()
            }),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadReckoningMessage {
    pub id: String,
    pub received_at: i64,
    pub position: PositionDr,
    pub trip: TripDescriptor,
    pub vehicle: VehicleDr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionDr {
    pub odometer: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDr {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeedEntity {
    pub id: String,
    pub vehicle: Option<VehiclePosition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehiclePosition {
    pub position: Option<Position>,
    pub trip: Option<TripDescriptor>,
    pub vehicle: Option<VehicleDescriptor>,
    pub occupancy_status: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub bearing: Option<f64>,
    pub speed: Option<f64>,
    pub odometer: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VehicleDescriptor {
    pub id: String,
    pub label: Option<String>,
    pub license_plate: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TripDescriptor {
    pub trip_id: String,
    pub route_id: String,
    pub start_time: Option<String>,
    pub start_date: Option<String>,
    pub direction_id: Option<i32>,
    pub schedule_relationship: Option<String>,
}

impl TripDescriptor {
    pub const ADDED: &'static str = "ADDED";
    pub const SCHEDULED: &'static str = "SCHEDULED";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extended_hours() {
        let tz = chrono_tz::Pacific::Auckland;
        let trip = TripInstance {
            trip_id: "trip".to_string(),
            route_id: "route".to_string(),
            service_date: "20240101".to_string(),
            start_time: "25:15:00".to_string(),
            end_time: String::new(),
            direction_id: None,
            is_added_trip: false,
            error: false,
        };
        let timestamp = timestamp(&trip, tz).unwrap();
        // 25:15 local time maps to 01:15 NZDT on the following day (UTC+13), which is
        // 12:15 UTC — 44_100 seconds from midnight.
        assert_eq!(timestamp % 86_400, 44_100);
    }
}
