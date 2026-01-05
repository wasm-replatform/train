use anyhow::Context as _;
use common::block_mgt;
use common::fleet::{self, Vehicle};
use warp_sdk::api::{Context, Handler, Reply};
use warp_sdk::{
    Config, Error, HttpRequest, Identity, Message, Publisher, Result, StateStore, bad_request,
};

use crate::gtfs::{self, StopType, StopTypeEntry};
use crate::trip_state::{self, VehicleInfo, VehicleTripInfo};
use crate::types::{DilaxMessage, EnrichedEvent};

const STOP_SEARCH_DISTANCE_METERS: u32 = 150;
const DILAX_ENRICHED_TOPIC: &str = "realtime-dilax-apc-enriched.v2";

/// Dilax empty response.
#[derive(Debug, Clone)]
pub struct DilaxReply;

async fn handle<P>(_owner: &str, request: DilaxMessage, provider: &P) -> Result<Reply<DilaxReply>>
where
    P: Config + HttpRequest + Publisher + StateStore + Identity,
{
    process(request, provider).await?;
    Ok(DilaxReply.into())
}

impl<P> Handler<P> for DilaxMessage
where
    P: Config + HttpRequest + Publisher + StateStore + Identity,
{
    type Error = Error;
    type Input = Vec<u8>;
    type Output = DilaxReply;

    // TODO: implement "owner"
    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<DilaxReply>> {
        handle(ctx.owner, self, ctx.provider).await
    }
}

/// Enriches a Dilax event with vehicle, stop, trip, and occupancy information.
///
/// # Errors
///
/// Returns an error when one of the providers or the key-value store reports a failure
/// while augmenting the incoming Dilax event.
pub async fn process<P>(event: DilaxMessage, provider: &P) -> Result<()>
where
    P: Config + HttpRequest + Publisher + StateStore + Identity,
{
    let vehicle_label = vehicle_label(&event)
        .ok_or_else(|| bad_request!("vehicle label missing for device {:?}", event.device))?;

    let vehicle = fleet::vehicle(&vehicle_label, provider)
        .await
        .map_err(|err| bad_request!("failed to resolve vehicle for label {vehicle_label}: {err}"))?
        .ok_or_else(|| bad_request!("vehicle not found for label {vehicle_label}"))?;

    let (vehicle_seating, vehicle_total) = vehicle_capacity(&vehicle)
        .ok_or_else(|| bad_request!("vehicle {} lacks capacity information", vehicle.id))?;
    let vehicle_id = vehicle.id.clone();

    let allocation = block_mgt::allocation(&vehicle_id, provider)
        .await
        .map_err(|err| {
            bad_request!("failed to fetch block allocation for vehicle {vehicle_id}: {err}")
        })?
        .ok_or_else(|| bad_request!("block allocation unavailable for vehicle {vehicle_id}"))?;

    let trip_id_value = allocation.trip_id.clone();
    let start_date_value = allocation.service_date.clone();
    let start_time_value = allocation.start_time.clone();
    tracing::debug!(vehicle_id = %vehicle_id, allocation = ?allocation, trip_id = %trip_id_value);

    let stop_id_value: String = stop_id(&vehicle_id, &event, provider).await?;

    trip_state::update_vehicle(
        &vehicle_id,
        Some(trip_id_value.as_str()),
        vehicle_seating,
        vehicle_total,
        &event,
        provider,
    )
    .await
    .map_err(|err| bad_request!("failed to update trip state for vehicle {vehicle_id}: {err}"))?;

    let vt = VehicleTripInfo {
        vehicle_info: VehicleInfo {
            vehicle_id: vehicle_id.clone(),
            label: Some(vehicle_label.clone()),
        },
        trip_id: Some(trip_id_value.clone()),
        stop_id: Some(stop_id_value.clone()),
        last_received_timestamp: Some(event.clock.utc.clone()),
        dilax_message: Some(event.clone()),
    };
    trip_state::set_trip(vt, provider).await.map_err(|err| {
        bad_request!("failed to persist trip info for vehicle {vehicle_id}: {err}")
    })?;

    let enriched = EnrichedEvent {
        event,
        stop_id: Some(stop_id_value),
        trip_id: Some(trip_id_value),
        start_date: Some(start_date_value),
        start_time: Some(start_time_value),
    };

    let payload = serde_json::to_vec(&enriched).context("serializing event")?;
    let mut message = Message::new(&payload);
    if let Some(trip_id) = &enriched.trip_id {
        message.headers.insert("key".to_string(), trip_id.clone());
    }

    Publisher::send(provider, DILAX_ENRICHED_TOPIC, &message).await?;

    Ok(())
}

fn vehicle_label(event: &DilaxMessage) -> Option<String> {
    let site = &event.device.as_ref()?.site;

    let (prefix, suffix) = site
        .strip_prefix("AM")
        .map(|suffix| ("AMP", suffix))
        .or_else(|| site.strip_prefix("AD").map(|suffix| ("ADL", suffix)))?;

    // train label: format as 14 characters
    let width = 14usize.saturating_sub(prefix.len());
    Some(format!("{prefix}{suffix:>width$}"))
}

fn vehicle_capacity(vehicle: &Vehicle) -> Option<(i64, i64)> {
    vehicle.capacity.as_ref().map(|capacity| (capacity.seating, capacity.total))
}

/// Resolve the GTFS stop identifier for the Dilax event waypoint.
///
/// # Errors
///
/// Returns an error when the waypoint is missing, provider requests fail, or no stop
/// matching the Dilax waypoint can be determined.
async fn stop_id<P>(vehicle_id: &str, event: &DilaxMessage, provider: &P) -> Result<String>
where
    P: Config + HttpRequest + Publisher + StateStore + Identity,
{
    let vehicle_id_owned = vehicle_id.to_string();

    let Some(waypoint) = event.wpt.as_ref() else {
        return Err(bad_request!(
            "dilax-adapter event missing waypoint data for vehicle {vehicle_id_owned}"
        ))?;
    };

    let stops =
        gtfs::location_stops(&waypoint.lat, &waypoint.lon, STOP_SEARCH_DISTANCE_METERS, provider)
            .await
            .map_err(|err| {
                bad_request!("failed to look up stops for vehicle {vehicle_id_owned}: {err}")
            })?;
    if stops.is_empty() {
        return Err(bad_request!("stop id unavailable for vehicle {vehicle_id_owned}"))?;
    }

    let stop_types = gtfs::stop_types(provider).await.map_err(|err| {
        bad_request!("failed to look up stop types for vehicle {vehicle_id_owned}: {err}")
    })?;
    if stop_types.is_empty() {
        return Err(bad_request!("train stop types unavailable for vehicle {vehicle_id_owned}"))?;
    }

    for stop in &stops {
        tracing::debug!(vehicle_id = %vehicle_id, stop = ?stop);

        if let Some(code) = stop.stop_code.as_deref()
            && is_station(&stop_types, code)
        {
            tracing::debug!(vehicle_id = %vehicle_id, stop_id = %stop.stop_id, stop_code = code);
            return Ok(stop.stop_id.clone());
        }
    }

    Err(bad_request!("stop id unavailable for vehicle {vehicle_id_owned}"))
}

fn is_station(stop_types: &[StopTypeEntry], stop_code: &str) -> bool {
    stop_types.iter().any(|entry| {
        entry.parent_stop_code.as_deref() == Some(stop_code)
            && entry.route_type == Some(StopType::Train as u32)
    })
}
