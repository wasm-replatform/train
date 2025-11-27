use std::collections::HashSet;
use std::sync::OnceLock;

use anyhow::{Context, anyhow};
use bytes::Bytes;
use http::header::AUTHORIZATION;
use http::{Method, Request, Uri};
use http_body_util::Empty;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, error, info, warn};
use urlencoding::encode;

use crate::config::Config;
use crate::error::{DomainError, Error, Result};
use crate::provider::{Provider, ProviderWrapper};
use crate::types::{
    DilaxEvent, DilaxEventEnriched, DilaxState, DilaxStateRecord, FleetVehicleResponse,
    FleetVehicleType, MessageToken, OccupancyStatus, PassengerCount, ServiceDate, ServiceTime,
    StopId, StopInfoRecord, TrainStopTypeRecord, TripId, VehicleAllocation, VehicleId, VehicleInfo,
    VehicleLabel, VehicleTripInfo,
};

const VEHICLE_LABEL_WIDTH: usize = 14;
const TRAIN_STOP_ROUTE_TYPE: i32 = 2;

pub async fn process_event<P>(
    wrapper: &ProviderWrapper<'_, P>, event: DilaxEvent,
) -> Result<Option<DilaxEventEnriched>>
where
    P: Provider + ?Sized,
{
    process_event_internal(wrapper, event)
        .await
        .map_err(|err| DomainError::ProcessingError(format!("Dilax processing failed: {err}")))
        .map_err(Error::from)
}

async fn process_event_internal<P>(
    wrapper: &ProviderWrapper<'_, P>, event: DilaxEvent,
) -> anyhow::Result<Option<DilaxEventEnriched>>
where
    P: Provider + ?Sized,
{
    let config = wrapper.config();

    let mut trip_id: Option<String> = None;
    let stop_id;
    let mut vehicle_id: Option<VehicleId> = None;
    let mut start_date: Option<String> = None;
    let mut start_time: Option<String> = None;
    let mut vehicle_seating_space: Option<u32> = None;
    let mut vehicle_total_space: Option<u32> = None;

    let vehicle_label = get_vehicle_label(&event);
    if vehicle_label.is_none() {
        warn!(
            "Could not get a valid vehicle label from the dilax-adapter event, skipping... event = {:?}",
            event
        );
    }

    if let Some(label) = &vehicle_label {
        if let Some(vehicle_info) = fetch_vehicle_info(wrapper, config, label).await? {
            let vehicle_info_json = serde_json::to_string(&vehicle_info).unwrap_or_default();
            info!("vehicleInfo [{}]", vehicle_info_json);

            let vehicle_identifier = VehicleId::new(vehicle_info.id.clone());
            vehicle_id = Some(vehicle_identifier.clone());
            info!("vehicleId [{}] vehicleLabel [{}]", vehicle_identifier.as_str(), label.as_str());

            if let Some(capacity) = vehicle_info.capacity.as_ref() {
                vehicle_seating_space = capacity.seating;
                vehicle_total_space = capacity.total;
                if vehicle_seating_space.is_none() || vehicle_total_space.is_none() {
                    warn!(
                        "vehicleId [{}] Could not get vehicle capcacity for vehicleLabel [{}], skipping...",
                        vehicle_identifier.as_str(),
                        label.as_str()
                    );
                    return Ok(None);
                }
                info!(
                    "vehicleId [{}] vehicleTotalSpace [{:?}] vehicleSeatingSpace [{:?}]",
                    vehicle_identifier.as_str(),
                    vehicle_total_space,
                    vehicle_seating_space
                );
            } else {
                warn!(
                    "vehicleId [{}] Could not get vehicle capcacity for vehicleLabel [{}], skipping...",
                    vehicle_identifier.as_str(),
                    label.as_str()
                );
                return Ok(None);
            }

            if let Some(allocation) = fetch_allocation(wrapper, config, &vehicle_identifier).await?
            {
                info!(
                    "vehicleId [{}] vehicleAllocation [{}]",
                    vehicle_identifier.as_str(),
                    serde_json::to_string(&allocation)?
                );
                trip_id = allocation.trip_id.clone();
                start_date = allocation.service_date.clone();
                start_time = allocation.start_time.clone();
            } else {
                warn!("vehicleId [{}] Failed to find allocated trip", vehicle_identifier.as_str());
            }
        } else {
            warn!("Failed to get vehicleId from vehicleLabel [{}]", label.as_str());
        }
    }

    stop_id = get_train_stop_id(wrapper, config, vehicle_id.as_ref(), &event).await?;
    if stop_id.is_none() {
        warn!(
            "vehicleId [{}] Failed to find a stop with the dilax-adapter event",
            vehicle_id.as_ref().map_or("", VehicleId::as_str)
        );
    }

    info!(
        "vehicleId [{}] tripId [{:?}] stopId [{:?}]",
        vehicle_id.as_ref().map_or("", VehicleId::as_str),
        trip_id,
        stop_id
    );

    let mut enriched = DilaxEventEnriched::new(event.clone());
    enriched.stop_id = stop_id.clone().map(StopId::from);
    enriched.trip_id = trip_id.clone().map(|value| value.into());
    enriched.start_date = start_date.clone().map(ServiceDate::from);
    enriched.start_time = start_time.clone().map(ServiceTime::from);

    let Some(vehicle_id) = vehicle_id else {
        warn!(
            "vehicleId [{}] Failed to find a vehicleId to process passenger count. skipping...",
            "null"
        );
        return Ok(Some(enriched));
    };

    let vehicle_label_ref = vehicle_label.as_ref().map(|label| label.as_str());

    let seating = vehicle_seating_space.ok_or_else(|| anyhow!("missing seating capacity"))?;
    let total = vehicle_total_space.ok_or_else(|| anyhow!("missing total capacity"))?;

    process_passenger_counts(
        wrapper,
        config,
        &vehicle_id,
        vehicle_label_ref,
        trip_id.as_deref(),
        stop_id.as_deref(),
        &event,
        seating,
        total,
    )
    .await?;

    Ok(Some(enriched))
}

async fn fetch_vehicle_info<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, label: &VehicleLabel,
) -> anyhow::Result<Option<FleetVehicleResponse>>
where
    P: Provider + ?Sized,
{
    let encoded_label = encode(label.as_str());
    let url = format!("{}/vehicles?label={encoded_label}", config.fleet_api_url);
    let request = Request::builder()
        .method(Method::GET)
        .uri(url.parse::<Uri>().context("parsing Fleet API URI")?)
        .body(Empty::<Bytes>::new())
        .context("building Fleet API request")?;

    let response = wrapper.send_http(request).await.context("fetching fleet vehicle")?;
    if !response.status().is_success() {
        warn!(
            "Fleet API responded with status {} for vehicleLabel [{}]",
            response.status(),
            label.as_str()
        );
        return Ok(None);
    }

    let body = response.into_body();
    let vehicles: Vec<FleetVehicleResponse> =
        serde_json::from_slice(&body).context("deserializing fleet response")?;
    Ok(select_train_vehicle(vehicles))
}

fn select_train_vehicle(vehicles: Vec<FleetVehicleResponse>) -> Option<FleetVehicleResponse> {
    vehicles
        .into_iter()
        .find(|vehicle| matches!(vehicle.r#type.as_ref(), Some(FleetVehicleType { r#type: Some(value) }) if value.eq_ignore_ascii_case("train")))
}

async fn fetch_allocation<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: &VehicleId,
) -> anyhow::Result<Option<VehicleAllocation>>
where
    P: Provider + ?Sized,
{
    let token = wrapper.access_token().await.context("retrieving access token")?;
    let url = format!(
        "{}/allocations/vehicles/{}?currentTrip=true",
        config.block_mgt_api_url,
        encode(vehicle_id.as_str())
    );
    let request = Request::builder()
        .method(Method::GET)
        .uri(url.parse::<Uri>().context("parsing block management URI")?)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Empty::<Bytes>::new())
        .context("building block management request")?;

    let response =
        wrapper.send_http(request).await.context("fetching block management allocation")?;
    if !response.status().is_success() {
        warn!(
            "vehicleId [{}] block management responded with status {}",
            vehicle_id.as_str(),
            response.status()
        );
        return Ok(None);
    }

    let body = response.into_body();
    if body.is_empty() {
        return Ok(None);
    }

    let parsed: Value =
        serde_json::from_slice(&body).context("deserializing block management envelope")?;
    if let Some(current) = parsed.get("current") {
        let allocations: Vec<VehicleAllocation> =
            serde_json::from_value(current.clone()).context("deserializing vehicle allocation")?;
        Ok(allocations.into_iter().next())
    } else {
        Ok(None)
    }
}

async fn get_train_stop_id<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: Option<&VehicleId>,
    event: &DilaxEvent,
) -> anyhow::Result<Option<String>>
where
    P: Provider + ?Sized,
{
    let Some(wpt) = &event.wpt else {
        warn!(
            "vehicleId [{}] No lon and lat in event, not finding stopId",
            vehicle_id.map_or("", VehicleId::as_str)
        );
        return Ok(None);
    };

    info!(
        "vehicleId [{}] Stop coordinate: lat [{}] lon {}",
        vehicle_id.map_or("", VehicleId::as_str),
        wpt.lat,
        wpt.lon
    );

    let stops = fetch_stops_by_location(wrapper, config, &wpt.lat, &wpt.lon).await?;
    if stops.is_empty() {
        return Ok(None);
    }

    let train_stop_types = fetch_train_stop_types(wrapper, config).await?;
    if let Some(message) = train_stop_types.message {
        error!(
            "vehicleId [{}] Failed to get train stop types cannot determine if stop is a station [{}]",
            vehicle_id.map_or("", VehicleId::as_str),
            message
        );
        return Ok(None);
    }

    let train_stop_codes: HashSet<String> = train_stop_types
        .records
        .into_iter()
        .filter_map(|record| match (record.parent_stop_code, record.route_type) {
            (Some(code), Some(route_type)) if route_type == TRAIN_STOP_ROUTE_TYPE => Some(code),
            _ => None,
        })
        .collect();

    for stop in stops {
        debug!(
            "vehicleId [{}] stopInfo [{}]",
            vehicle_id.map_or("", VehicleId::as_str),
            serde_json::to_string(&stop)?
        );
        if train_stop_codes.contains(&stop.stop_code) {
            info!(
                "vehicleId [{}] Found a matching train stop: stopId [{}] stopCode [{}]",
                vehicle_id.map_or("", VehicleId::as_str),
                stop.stop_id,
                stop.stop_code
            );
            return Ok(Some(stop.stop_id));
        }
    }

    Ok(None)
}

struct TrainStopTypes {
    records: Vec<TrainStopTypeRecord>,
    message: Option<String>,
}

async fn fetch_train_stop_types<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config,
) -> anyhow::Result<TrainStopTypes>
where
    P: Provider + ?Sized,
{
    let request = Request::builder()
        .method(Method::GET)
        .uri(
            format!("{}/stopstypes/", config.gtfs_static_api_url)
                .parse::<Uri>()
                .context("parsing GTFS URI")?,
        )
        .body(Empty::<Bytes>::new())
        .context("building GTFS request")?;

    let response = wrapper.send_http(request).await.context("fetching GTFS stop types")?;
    if !response.status().is_success() {
        warn!("GTFS stop types responded with status {}", response.status());
        return Ok(TrainStopTypes {
            records: Vec::new(),
            message: Some(format!("HTTP {}", response.status())),
        });
    }

    let body = response.into_body();
    if body.is_empty() {
        return Ok(TrainStopTypes { records: Vec::new(), message: None });
    }

    match serde_json::from_slice::<Value>(&body).context("deserializing GTFS stop types")? {
        Value::Array(items) => {
            let mut records = Vec::with_capacity(items.len());
            for value in items {
                let record: TrainStopTypeRecord = serde_json::from_value(value)
                    .context("deserializing train stop type record")?;
                records.push(record);
            }
            Ok(TrainStopTypes { records, message: None })
        }
        Value::Object(obj) => {
            let message = obj.get("message").and_then(Value::as_str).map(|s| s.to_string());
            Ok(TrainStopTypes { records: Vec::new(), message })
        }
        _ => Ok(TrainStopTypes { records: Vec::new(), message: None }),
    }
}

async fn fetch_stops_by_location<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, lat: &str, lon: &str,
) -> anyhow::Result<Vec<StopInfoRecord>>
where
    P: Provider + ?Sized,
{
    let uri = format!(
        "{}/gtfs/stops/geosearch?lat={}&lng={}&distance={}",
        config.cc_static_api_url,
        encode(lat),
        encode(lon),
        config.stop_search_radius_meters
    );
    let request = Request::builder()
        .method(Method::GET)
        .uri(uri.parse::<Uri>().context("parsing CC Static URI")?)
        .body(Empty::<Bytes>::new())
        .context("building CC Static request")?;

    let response = wrapper.send_http(request).await.context("fetching stops info")?;
    if !response.status().is_success() {
        warn!(
            "Failed to get stop info by lat [{}] lng [{}]: status {}",
            lat,
            lon,
            response.status()
        );
        return Ok(Vec::new());
    }

    let body = response.into_body();
    if body.is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_slice(&body).context("deserializing stop info")
}

async fn process_passenger_counts<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: &VehicleId,
    vehicle_label: Option<&str>, trip_id: Option<&str>, stop_id: Option<&str>, event: &DilaxEvent,
    seating_space: u32, total_space: u32,
) -> anyhow::Result<()>
where
    P: Provider + ?Sized,
{
    let state_key = format!("{}:{}", config.redis.apc_vehicle_id_state_key, vehicle_id.as_str());
    let (mut state, previous_state_string) = load_state(wrapper, config, vehicle_id).await?;

    let token_value: u64 = event
        .clock
        .utc
        .parse()
        .map_err(|err| anyhow!("invalid token {}: {err}", event.clock.utc))?;
    let token = MessageToken::new(token_value);

    if token.value() <= state.token.value() {
        warn!(
            "vehicleId [{}] token [{}] < dilaxState.token [{}], skipping dup msg",
            vehicle_id.as_str(),
            token.value(),
            state.token.value()
        );
    } else {
        state.token = token;
        let has_trip_id_changed = detect_trip_change(&mut state, trip_id, vehicle_id);

        if trip_id.is_none() || has_trip_id_changed {
            state.count = PassengerCount::ZERO;
            warn!(
                "vehicleId [{}] Reset running count for tripId [{:?}]",
                vehicle_id.as_str(),
                trip_id
            );
            update_running_count(event, &mut state, vehicle_id, true);
        } else {
            update_running_count(event, &mut state, vehicle_id, false);
        }

        update_occupancy_status(&mut state, vehicle_id, seating_space, total_space);

        let state_record = DilaxStateRecord::from(&state);
        let state_json = serde_json::to_vec(&state_record).context("serializing dilax state")?;
        let previous = wrapper
            .state_set(&state_key, &state_json, Some(config.apc_ttl_secs))
            .await
            .context("upserting dilax state")?;

        if let (Some(prev), Some(prev_string)) = (previous, previous_state_string) {
            if prev.as_slice() != prev_string.as_bytes() {
                warn!(
                    "vehicleId [{}] overwritten by another process during this update! dilaxStatePrevStr [{}] lastValue [{}] dilaxState [{}]",
                    vehicle_id.as_str(),
                    prev_string,
                    String::from_utf8_lossy(&prev),
                    String::from_utf8_lossy(&state_json)
                );
            }
        }

        if let Some(occupancy_status) = state_record.occupancy_status {
            let key = format!("{}:{}", config.redis.key_occupancy, vehicle_id.as_str());
            if let Err(err) = wrapper
                .state_set(
                    &key,
                    occupancy_status.value().to_string().as_bytes(),
                    Some(config.legacy_redis_ttl_secs),
                )
                .await
            {
                error!(
                    "vehicleId [{}] Failed to cache the occupancy status: {}",
                    vehicle_id.as_str(),
                    err
                );
            }
        }

        if let Err(err) = wrapper
            .state_set(
                &format!("{}:{}", config.redis.apc_vehicle_id_key, vehicle_id.as_str()),
                state.count.value().to_string().as_bytes(),
                Some(config.apc_ttl_secs),
            )
            .await
        {
            error!(
                "vehicleId [{}] Failed to cache the occupancy count: {}",
                vehicle_id.as_str(),
                err
            );
        }
    }

    if let Err(err) =
        save_vehicle_trip_info(wrapper, config, vehicle_id, vehicle_label, trip_id, stop_id, event)
            .await
    {
        error!("Failed to save VehicleTripInfo for {}:{:?}: {}", vehicle_id.as_str(), trip_id, err);
    }

    Ok(())
}

async fn load_state<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: &VehicleId,
) -> anyhow::Result<(DilaxState, Option<String>)>
where
    P: Provider + ?Sized,
{
    let state_key = format!("{}:{}", config.redis.apc_vehicle_id_state_key, vehicle_id.as_str());
    if let Some(bytes) = wrapper.state_get(&state_key).await.context("reading dilax state")? {
        let previous = String::from_utf8(bytes.clone()).context("utf8 decoding dilax state")?;
        match serde_json::from_str::<DilaxStateRecord>(&previous) {
            Ok(record) => Ok((record.into(), Some(previous))),
            Err(e) => {
                error!("[Dilax] deserializing dilax state failed: {}\nstate json: {}", e, previous);
                Err(anyhow!("deserializing dilax state: {e}"))
            }
        }
    } else {
        let mut state = DilaxState::from(DilaxStateRecord::default());
        perform_backward_compatibility_migration(wrapper, config, vehicle_id, &mut state).await?;
        Ok((state, None))
    }
}

async fn perform_backward_compatibility_migration<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: &VehicleId,
    state: &mut DilaxState,
) -> anyhow::Result<()>
where
    P: Provider + ?Sized,
{
    let migrated_key =
        format!("{}:{}", config.redis.apc_vehicle_id_migrated_key, vehicle_id.as_str());
    if wrapper.state_get(&migrated_key).await.context("reading migration flag")?.is_some() {
        return Ok(());
    }

    let trip_key = format!("{}:{}", config.redis.apc_vehicle_trip_key, vehicle_id.as_str());
    if let Some(bytes) = wrapper.state_get(&trip_key).await.context("reading legacy trip key")? {
        let trip = String::from_utf8(bytes).context("utf8 decoding legacy trip")?;
        warn!(
            "vehicleId [{}] migrating previous version tripIdOldVersion [{}]",
            vehicle_id.as_str(),
            trip
        );
        state.last_trip_id = Some(trip.into());
    }

    let count_key = format!("{}:{}", config.redis.apc_vehicle_id_key, vehicle_id.as_str());
    if let Some(bytes) = wrapper.state_get(&count_key).await.context("reading legacy count key")? {
        let count_value = String::from_utf8(bytes).context("utf8 decoding legacy count")?;
        if let Ok(count) = count_value.parse::<u32>() {
            warn!(
                "vehicleId [{}] migrating previous version countOldVersion [{}]",
                vehicle_id.as_str(),
                count
            );
            state.count = PassengerCount::new(count);
        }
    }

    wrapper.state_set(&migrated_key, b"true", None).await.context("writing migration flag")?;

    Ok(())
}

fn detect_trip_change(
    state: &mut DilaxState, trip_id: Option<&str>, vehicle_id: &VehicleId,
) -> bool {
    let mut has_changed = false;
    if let Some(current_trip_id) = trip_id {
        let previous = state.last_trip_id.clone();
        match previous.as_ref() {
            Some(last) if last.as_str() != current_trip_id => {
                has_changed = true;
                state.last_trip_id = Some(TripId::new(current_trip_id));
            }
            None => {
                state.last_trip_id = Some(TripId::new(current_trip_id));
            }
            _ => {}
        }
        let last_trip_display = previous.as_ref().map(|value| value.as_str()).unwrap_or("null");
        info!(
            "vehicleId [{}] lastTripId [{}] tripId [{}] hasTripIdChanged [{}]",
            vehicle_id.as_str(),
            last_trip_display,
            current_trip_id,
            has_changed
        );
    }
    has_changed
}

fn update_running_count(
    event: &DilaxEvent, state: &mut DilaxState, vehicle_id: &VehicleId, skip_out: bool,
) {
    let mut in_total = 0_u32;
    let mut out_total = 0_u32;
    let mut out_total_no_skip = 0_u32;

    for door in &event.doors {
        in_total += door.r#in;
        if !skip_out {
            out_total += door.out;
        }
        out_total_no_skip += door.out;
    }

    info!(
        "vehicleId [{}] Running count: inTotal [{}] outTotal [{}] outTotalNoSkip [{}] skipOut [{}]",
        vehicle_id.as_str(),
        in_total,
        out_total,
        out_total_no_skip,
        skip_out
    );

    let previous_count = state.count.value();
    let decreased = previous_count.saturating_sub(out_total);
    let current_count = decreased.saturating_add(in_total);
    if (previous_count as i64) - (out_total as i64) + (in_total as i64) < 0 {
        warn!("vehicleId [{}] has -ve currentCount [{}]", vehicle_id.as_str(), current_count);
    }
    state.count = PassengerCount::new(current_count);
    info!("vehicleId [{}] dilaxState.count [{}]", vehicle_id.as_str(), state.count.value());
}

fn update_occupancy_status(
    state: &mut DilaxState, vehicle_id: &VehicleId, seating_space: u32, total_space: u32,
) {
    let count = state.count.value();
    let seating = seating_space as f64;
    let total = total_space as f64;
    let status = if count < (seating * 0.05).trunc() as u32 {
        OccupancyStatus::Empty
    } else if count < (seating * 0.4).trunc() as u32 {
        OccupancyStatus::ManySeatsAvailable
    } else if count < (seating * 0.9).trunc() as u32 {
        OccupancyStatus::FewSeatsAvailable
    } else if count < (total * 0.9).trunc() as u32 {
        OccupancyStatus::StandingRoomOnly
    } else {
        OccupancyStatus::Full
    };

    state.occupancy_status = Some(status);
    info!("vehicleId [{}] occupancyStatus [{:?}]", vehicle_id.as_str(), status.code().value());
}

async fn save_vehicle_trip_info<P>(
    wrapper: &ProviderWrapper<'_, P>, config: &Config, vehicle_id: &VehicleId,
    vehicle_label: Option<&str>, trip_id: Option<&str>, stop_id: Option<&str>, event: &DilaxEvent,
) -> anyhow::Result<()>
where
    P: Provider + ?Sized,
{
    let info = VehicleTripInfo {
        vehicle_info: VehicleInfo {
            vehicle_id: vehicle_id.as_str().to_string(),
            label: vehicle_label.map(|v| v.to_string()),
        },
        trip_id: trip_id.map(|v| v.to_string()),
        stop_id: stop_id.map(|v| v.to_string()),
        dilax_message: Some(event.clone()),
        last_received_timestamp: Some(event.clock.utc.clone()),
    };
    let serialized = serde_json::to_vec(&info).context("serializing vehicle trip info")?;
    let key = format!("{}:{}", config.redis.key_vehicle_trip_info, vehicle_id.as_str());
    wrapper
        .state_set(&key, &serialized, Some(config.vehicle_trip_info_ttl_secs))
        .await
        .context("writing vehicle trip info")?;
    Ok(())
}

fn get_vehicle_label(event: &DilaxEvent) -> Option<VehicleLabel> {
    const VEHICLE_LABEL_MAP: &[(&str, &str)] = &[("AM", "AMP"), ("AD", "ADL")];
    let site = event.device.site.trim();
    if site.is_empty() {
        return None;
    }

    static LABEL_SPLIT: OnceLock<Regex> = OnceLock::new();
    let pattern =
        LABEL_SPLIT.get_or_init(|| Regex::new(r"\D+|\d+").expect("valid vehicle label pattern"));
    let parts: Vec<&str> = pattern.find_iter(site).map(|m| m.as_str()).collect();
    if parts.is_empty() {
        return None;
    }

    let alpha = parts.first()?.trim();
    let numeric = parts.iter().skip(1).fold(String::new(), |mut acc, part| {
        acc.push_str(part.trim());
        acc
    });

    if alpha.is_empty() || numeric.is_empty() {
        return None;
    }

    let mapped = VEHICLE_LABEL_MAP
        .iter()
        .find_map(|(key, value)| if alpha.eq_ignore_ascii_case(key) { Some(*value) } else { None })
        .unwrap_or(alpha);

    let padding = VEHICLE_LABEL_WIDTH.saturating_sub(mapped.len() + numeric.len());
    let label = format!("{}{}{}", mapped, " ".repeat(padding), numeric);
    Some(VehicleLabel::new(label))
}
