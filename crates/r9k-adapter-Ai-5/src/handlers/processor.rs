use anyhow::Context;
use chrono::{NaiveDate, TimeZone, Utc, DateTime};
use quick_xml::de::from_str;
use tracing::{debug, info};

use crate::{block_mgt, config, Error, R9kMessage, MovementType, Provider, Result, SmarTrakEvent, TrainUpdate};
use crate::gtfs::{self, StopInfo};
use crate::types::ChangeType;
use realtime::{Message as OutMessage, Publisher};

const OUTPUT_TOPIC: &str = "realtime-r9k-to-smartrak.v1";

/// Process and publish events derived from an inbound R9K XML payload.
///
/// # Errors
/// Returns domain errors for invalid XML, missing update data, timing thresholds, or I/O failures.
///
/// # Panics
/// Function does not intentionally panic; any panic would indicate unrecoverable parsing assumptions.
pub async fn process(xml_payload: &str, provider: &impl Provider) -> Result<()> {
    // Attempt flexible parsing: first as full envelope (R9kMessage), then directly as TrainUpdate root.
    let train_update: TrainUpdate = match from_str::<R9kMessage>(xml_payload) {
        Ok(msg) if msg.train_update.is_some() => msg.train_update.unwrap(),
        _ => match from_str::<TrainUpdate>(xml_payload) {
            Ok(tu) => tu,
            Err(err) => {
                // If the payload contains a <CCO> wrapper, try to extract inner <ActualizarDatosTren> segment.
                if let Some(start) = xml_payload.find("<ActualizarDatosTren>") {
                    if let Some(end) = xml_payload.find("</ActualizarDatosTren>") {
                        let inner = &xml_payload[start..end + "</ActualizarDatosTren>".len()];
                        match from_str::<TrainUpdate>(inner) {
                            Ok(tu_inner) => tu_inner,
                            Err(inner_err) => return Err(Error::InvalidFormat(format!("xml parse error: {err}; inner: {inner_err}"))),
                        }
                    } else { return Err(Error::InvalidFormat(format!("xml parse error: {err}"))); }
                } else { return Err(Error::InvalidFormat(format!("xml parse error: {err}"))); }
            }
        },
    };

    if train_update.changes.is_empty() { return Err(Error::NoUpdate); }

    let primary_change = &train_update.changes[0];
    let movement_type = movement_type(primary_change.change_type);

    // Filter non-progress OTHER events (legacy rule)
    if train_update.changes.len() == 1 && matches!(movement_type, MovementType::Other) { return Ok(()); }

    // Validate timing
    validate_delay(&train_update)?;

    // Station relevance
    let station_map = config::station_id_to_stop_code_map();
    let stop_code = station_map.get(&primary_change.station).unwrap_or(&"unmapped");
    if *stop_code == "unmapped" { return Ok(()); }
    let filter_ok = config::filter_stations().iter().any(|s| station_map.get(&s.parse::<i32>().unwrap_or_default()) == Some(stop_code));
    if !filter_ok { return Ok(()); }

    // Fetch GTFS stops for coordinates
    let stops = gtfs::stops(provider).await.context("fetching gtfs stops")?;
    let stop_info_opt = stops.iter().find(|s| s.stop_code == *stop_code).cloned();

    // Overwrite departure location if applicable
    let overwrite_map = config::departure_location_overwrite();
    let use_overwrite = !matches!(movement_type, MovementType::Arrival) && stop_code.parse::<i32>().ok().and_then(|c| overwrite_map.get(&c)).is_some();
    let (lat, lon) = if use_overwrite { let c = stop_code.parse::<i32>().unwrap(); *overwrite_map.get(&c).unwrap() } else { match stop_info_opt { Some(StopInfo{stop_lat, stop_lon, ..}) => (stop_lat, stop_lon), None => return Ok(()), } };

    // Vehicles
    let ref_id = train_update.even_train_id.clone().filter(|v| !v.is_empty()).or_else(|| train_update.odd_train_id.clone()).ok_or_else(|| Error::ProcessingError("missing train id".to_string()))?;
    let vehicles = block_mgt::vehicles_by_external_ref_id(&ref_id, provider).await.context("fetching vehicles")?;
    if vehicles.is_empty() { return Ok(()); }

    let received_at = Utc::now();
    for label in vehicles {
        let mut event = SmarTrakEvent::new(&label, lat, lon, received_at);
        publish_two_tap(&mut event, provider).await?;
    }

    Ok(())
}

const fn movement_type(change_type: i32) -> MovementType {
    match change_type { 
        x if x == ChangeType::ReachedFinalDestination as i32 || x == ChangeType::ArrivedAtStation as i32 => MovementType::Arrival,
        x if x == ChangeType::ExitedFirstStation as i32 || x == ChangeType::ExitedStation as i32 || x == ChangeType::PassedStationWithoutStopping as i32 => MovementType::Departure,
        x if x == ChangeType::ScheduleChange as i32 => MovementType::Prediction,
        _ => MovementType::Other,
    }
}

fn validate_delay(train_update: &TrainUpdate) -> Result<()> {
    let c = &train_update.changes[0];
    let event_seconds = if c.has_departed { c.actual_departure_time } else if c.has_arrived { c.actual_arrival_time } else { -1 };
    if event_seconds <= 0 { return Err(Error::NoActualUpdate); }

    // Allow tests to bypass time-based validation after actual time check.
    if std::env::var("R9K_SKIP_DELAY_VALIDATION").ok().as_deref() == Some("1") {
        return Ok(());
    }

    let date_str = train_update.created_date.clone().ok_or_else(|| Error::ProcessingError("missing createdDate".to_string()))?;
    let parts: Vec<&str> = date_str.split('/').collect();
    if parts.len() != 3 { return Err(Error::InvalidFormat("invalid date format".to_string())); }
    let day: u32 = parts[0].parse().map_err(|e| Error::InvalidFormat(format!("day: {e}")))?;
    let month: u32 = parts[1].parse().map_err(|e| Error::InvalidFormat(format!("month: {e}")))?;
    let year: i32 = parts[2].parse().map_err(|e| Error::InvalidFormat(format!("year: {e}")))?;
    let naive = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| Error::InvalidFormat("bad date".to_string()))?.and_hms_opt(0,0,0).ok_or_else(|| Error::InvalidFormat("bad time".to_string()))?;
    let tz_name = config::timezone();
    let tz: chrono_tz::Tz = tz_name.parse().unwrap_or(chrono_tz::Pacific::Auckland);
    let event_midnight = tz.from_local_datetime(&naive).single().ok_or_else(|| Error::InvalidFormat("tz".to_string()))?.with_timezone(&Utc);

    // Deterministic test override: if R9K_FIXED_NOW_TIMESTAMP (RFC3339) or R9K_FIXED_NOW_UNIX (seconds) provided, use that instead of Utc::now().
    let now_ts = if let Ok(rfc) = std::env::var("R9K_FIXED_NOW_TIMESTAMP") {
        DateTime::parse_from_rfc3339(&rfc).map(|dt| dt.with_timezone(&Utc)).map_err(|e| Error::InvalidFormat(format!("invalid fixed now rfc3339: {e}")))?
    } else if let Ok(unix) = std::env::var("R9K_FIXED_NOW_UNIX") {
        let secs: i64 = unix.parse().map_err(|e| Error::InvalidFormat(format!("invalid fixed now unix: {e}")))?;
        chrono::DateTime::<Utc>::from_timestamp(secs, 0).ok_or_else(|| Error::InvalidFormat("invalid unix ts".to_string()))?
    } else {
        Utc::now()
    };

    let message_delay = now_ts.timestamp() - (event_midnight.timestamp() + event_seconds);
    debug!(delay_seconds = message_delay, now = now_ts.to_rfc3339(), event_midnight = ?event_midnight, event_seconds, "r9k message delay computed");
    if message_delay > config::max_message_delay() { return Err(Error::Outdated(format!("delay seconds: {message_delay}"))); }
    if message_delay < config::min_message_delay() { return Err(Error::WrongTime(format!("delay seconds: {message_delay}"))); }
    Ok(())
}

async fn publish_two_tap(event: &mut SmarTrakEvent, provider: &impl Provider) -> Result<()> {
    // First publish after configurable delay (default 5000ms), then again after another delay.
    // Environment override enables fast unit tests: set R9K_TWO_TAP_DELAY_MS to small value.
    let delay_ms: u64 = std::env::var("R9K_TWO_TAP_DELAY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000);
    let delay = std::time::Duration::from_millis(delay_ms);
    // Capture base timestamp once to allow legacy-style deterministic increments if needed.
    let base_ts = chrono::Utc::now();
    let delay_ms_i64 = i64::try_from(delay_ms).unwrap_or(5000);
    tokio::time::sleep(delay).await;
    event.message_data.timestamp = (base_ts + chrono::TimeDelta::milliseconds(delay_ms_i64)).to_rfc3339();
    publish(event, provider).await?;
    tokio::time::sleep(delay).await;
    event.message_data.timestamp = (base_ts + chrono::TimeDelta::milliseconds(delay_ms_i64 * 2)).to_rfc3339();
    publish(event, provider).await?;
    Ok(())
}

async fn publish(event: &SmarTrakEvent, provider: &impl Provider) -> Result<()> {
    let payload = serde_json::to_vec(event).context("serializing smartrak event")?;
    let mut msg = OutMessage::new(&payload);
    if let Some(key) = &event.remote_data.external_id { msg.headers.insert("key".to_string(), key.clone()); }
    Publisher::send(provider, OUTPUT_TOPIC, &msg).await.context("publishing smartrak event")?;
    info!(label = ?event.remote_data.external_id, "published smartrak event");
    Ok(())
}
