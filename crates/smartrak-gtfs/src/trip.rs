use std::env;

use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{Duration, NaiveDate, TimeZone, Timelike};
use chrono_tz::Tz;
use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::{Method, StatusCode};
use http_body_util::Full;
use serde_json::Value;
use tracing::warn;

use crate::models::TripInstance;
use crate::{HttpRequest, Provider};

const CACHE_DIRECTIVE_PRIMARY: &str = "max-age=20, stale-if-error=10";

/// Retrieves the trip instance that matches the exact `trip_id`, `service_date`, and
/// `start_time` combination.
///
/// # Errors
///
/// Returns an error when the Trip Management API request fails or the response payload cannot
/// be deserialized.
pub async fn get_trip_instance(
    provider: &impl Provider, trip_id: &str, service_date: &str, start_time: &str,
) -> Result<Option<TripInstance>> {
    let trips = fetch_trips(provider, trip_id, service_date).await?;
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
pub async fn get_nearest_trip_instance(
    provider: &impl Provider, trip_id: &str, event_timestamp: i64,
) -> Result<Option<TripInstance>> {
    let tz = chrono_tz::Pacific::Auckland;
    let Some(event_dt) = tz.timestamp_opt(event_timestamp, 0).single() else {
        return Ok(None);
    };

    let current_date = event_dt.format("%Y%m%d").to_string();
    let mut trips = fetch_trips(provider, trip_id, &current_date).await?;

    if trips.first().is_some_and(TripInstance::has_error) {
        return Ok(trips.into_iter().next());
    }

    if event_dt.hour() < 4 {
        let previous_date = (event_dt - Duration::days(1)).format("%Y%m%d").to_string();
        let previous = fetch_trips(provider, trip_id, &previous_date).await?;
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

async fn fetch_trips(
    http: &impl HttpRequest, trip_id: &str, service_date: &str,
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

    decode_trip_instances(&body)
        .with_context(|| format!("deserializing trip instances for {trip_id} on {service_date}"))
}

fn decode_trip_instances(payload: &[u8]) -> Result<Vec<TripInstance>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let value: Value = serde_json::from_slice(payload).context("parsing trip payload")?;
    extract_trip_instances(value)
}

fn extract_trip_instances(value: Value) -> Result<Vec<TripInstance>> {
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
                return extract_trip_instances(data);
            }

            if let Some(data) = map.remove("data") {
                return extract_trip_instances(data);
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
    let trip_ts = trip_timestamp(trip, tz).unwrap_or(event_ts);
    (event_ts - trip_ts).abs()
}

fn trip_timestamp(trip: &TripInstance, tz: Tz) -> Option<i64> {
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
        let timestamp = trip_timestamp(&trip, tz).unwrap();
        // 25:15 local time maps to 01:15 NZDT on the following day (UTC+13), which is
        // 12:15 UTC — 44_100 seconds from midnight.
        assert_eq!(timestamp % 86_400, 44_100);
    }
}
