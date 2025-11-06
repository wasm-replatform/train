use std::borrow::ToOwned;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::error::Error;
use crate::occupancy::OccupancyStatus;
use crate::api::{BlockMgtProvider, CcStaticProvider, FleetProvider, GtfsStaticProvider};
use crate::state::DilaxState;
use crate::store::KvStore;
use crate::types::{DilaxEnrichedEvent, DilaxEvent, FleetVehicle, StopTypeEntry, VehicleTripInfo};

const VEHICLE_TRIP_INFO_TTL: Duration = Duration::from_secs(2 * 24 * 60 * 60);
const STOP_SEARCH_DISTANCE_METERS: u32 = 150;

#[derive(Clone)]
pub struct DilaxProcessor {
    config: Config,
    store: KvStore,
    fleet: Arc<dyn FleetProvider>,
    cc_static: Arc<dyn CcStaticProvider>,
    gtfs: Arc<dyn GtfsStaticProvider>,
    block: Arc<dyn BlockMgtProvider>,
}

impl DilaxProcessor {
    #[allow(clippy::too_many_arguments)]
    pub fn with_providers(
        config: Config, store: KvStore, fleet: Arc<dyn FleetProvider>,
        cc_static: Arc<dyn CcStaticProvider>, gtfs: Arc<dyn GtfsStaticProvider>,
        block: Arc<dyn BlockMgtProvider>,
    ) -> Self {
        Self { config, store, fleet, cc_static, gtfs, block }
    }

    pub async fn process(&self, event: DilaxEvent) -> Result<DilaxEnrichedEvent> {
        let mut trip_id: Option<String> = None;
        let mut stop_id: Option<String> = None;
        let mut start_date: Option<String> = None;
        let mut start_time: Option<String> = None;

        let vehicle_label = self.vehicle_label(&event);
        if vehicle_label.is_none() {
            warn!("Could not determine vehicle label from Dilax event: {:?}", event.device);
            return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
        }

        let vehicle = self.lookup_vehicle(vehicle_label.as_deref().unwrap()).await?;

        if vehicle.is_none() {
            warn!("Failed to resolve vehicle for label {vehicle_label:?}");
            return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
        }

        let vehicle = vehicle.unwrap();
        let vehicle_id = vehicle.id.clone();

        let (vehicle_seating, vehicle_total) = match Self::vehicle_capacity(&vehicle) {
            Some(capacity) => capacity,
            None => {
                warn!(
                    "Vehicle {vehicle_id} lacks capacity information; skipping passenger count processing"
                );
                return Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time });
            }
        };

        if let Some(allocation) = self.block.allocation_by_vehicle(&vehicle_id).await? {
            trip_id = Some(allocation.trip_id.clone());
            start_date = Some(allocation.service_date.clone());
            start_time = Some(allocation.start_time.clone());
            debug!(vehicle_id = %vehicle_id, allocation = ?allocation, trip_id = ?trip_id);
        } else {
            warn!(vehicle_id = %vehicle_id, vehicle_label = ?vehicle_label, "Failed to resolve block allocation");
        }

        stop_id = self.lookup_stop_id(&vehicle_id, &event).await?;

        if stop_id.is_none() {
            warn!(vehicle_id = %vehicle_id, "Unable to resolve stop ID from Dilax event");
        }

        self.update_vehicle_state(
            &vehicle_id,
            trip_id.as_deref(),
            vehicle_seating,
            vehicle_total,
            &event,
        )
        .await?;
        self.save_vehicle_trip_info(
            &vehicle_id,
            vehicle_label.as_deref(),
            trip_id.clone(),
            stop_id.clone(),
            &event,
        )
        .await?;

        Ok(DilaxEnrichedEvent { event, stop_id, trip_id, start_date, start_time })
    }

    fn vehicle_label(&self, event: &DilaxEvent) -> Option<String> {
        let site = event.device.site.trim();
        if site.is_empty() {
            return None;
        }

        let mut alpha = String::new();
        let mut numeric = String::new();
        for ch in site.chars() {
            if ch.is_ascii_digit() {
                numeric.push(ch);
            } else if ch.is_ascii_alphabetic() {
                alpha.push(ch);
            }
        }

        if alpha.is_empty() || numeric.is_empty() {
            return None;
        }

        let prefix = match alpha.as_str() {
            "AM" => "AMP".to_string(),
            "AD" => "ADL".to_string(),
            _ => alpha.clone(),
        };
        let padding_width = 14usize.saturating_sub(prefix.len() + numeric.len());
        let padded_prefix = format!("{prefix}{:padding$}", "", padding = padding_width);

        Some(format!("{padded_prefix}{numeric}"))
    }

    async fn lookup_vehicle(&self, label: &str) -> Result<Option<FleetVehicle>> {
        match self.fleet.train_by_label(label).await {
            Ok(vehicle) => Ok(vehicle),
            Err(error) => {
                error!(label = label, error = ?error, "Failed to query Fleet API");
                Err(error)
            }
        }
    }

    fn vehicle_capacity(vehicle: &FleetVehicle) -> Option<(i64, i64)> {
        vehicle.capacity.as_ref().map(|capacity| (capacity.seating, capacity.total))
    }

    async fn lookup_stop_id(&self, vehicle_id: &str, event: &DilaxEvent) -> Result<Option<String>> {
        let waypoint = match &event.wpt {
            Some(wpt) => wpt,
            None => {
                warn!(vehicle_id = %vehicle_id, "Dilax event missing waypoint data");
                return Ok(None);
            }
        };

        info!(vehicle_id = %vehicle_id, lat = %waypoint.lat, lon = %waypoint.lon, "Querying CC Static for stop info");
        let stops = self
            .cc_static
            .stops_by_location(&waypoint.lat, &waypoint.lon, STOP_SEARCH_DISTANCE_METERS)
            .await?;
        if stops.is_empty() {
            return Ok(None);
        }

        let train_stop_types = self.gtfs.train_stop_types().await?;
        if train_stop_types.is_empty() {
            warn!(vehicle_id = %vehicle_id, "GTFS train stop types unavailable");
            return Ok(None);
        }

        for stop in &stops {
            debug!(vehicle_id = %vehicle_id, stop = ?stop);
            if let Some(code) = stop.stop_code.as_deref() {
                if Self::is_train_station(&train_stop_types, code) {
                    info!(vehicle_id = %vehicle_id, stop_id = %stop.stop_id, stop_code = code);
                    return Ok(Some(stop.stop_id.clone()));
                }
            }
        }

        Ok(None)
    }

    fn is_train_station(train_stop_types: &[StopTypeEntry], stop_code: &str) -> bool {
        train_stop_types.iter().any(|entry| {
            entry.parent_stop_code.as_deref() == Some(stop_code)
                && entry.route_type == Some(crate::types::StopType::TrainStop as u32)
        })
    }

    async fn update_vehicle_state(
        &self, vehicle_id: &str, trip_id: Option<&str>, seating_capacity: i64, total_capacity: i64,
        event: &DilaxEvent,
    ) -> Result<()> {
        let state_key = format!("{}:{}", self.config.redis.apc_vehicle_id_state_key, vehicle_id);
        let state_prev = self.store.get_with_ttl(&state_key)?;
        let mut state = match state_prev.as_deref() {
            Some(raw) => serde_json::from_slice::<DilaxState>(raw).unwrap_or_default(),
            None => {
                let mut new_state = DilaxState::default();
                self.migrate_legacy_keys(vehicle_id, &mut new_state)?;
                new_state
            }
        };

        let token = match event.clock.utc.parse::<i64>() {
            Ok(value) => value,
            Err(_) => {
                warn!(vehicle_id = %vehicle_id, token = %event.clock.utc, "Unable to parse Dilax clock token");
                return Ok(());
            }
        };

        if token <= state.token {
            warn!(vehicle_id = %vehicle_id, token = token, last_token = state.token, "Received duplicate or out-of-order Dilax message");
            return Ok(());
        }
        state.token = token;

        let mut reset_running_count = false;
        if let Some(trip_id) = trip_id {
            match &state.last_trip_id {
                None => state.last_trip_id = Some(trip_id.to_string()),
                Some(last) if last != trip_id => {
                    reset_running_count = true;
                    state.last_trip_id = Some(trip_id.to_string());
                }
                _ => {}
            }
        } else {
            reset_running_count = true;
        }

        if reset_running_count {
            state.count = 0;
            warn!(vehicle_id = %vehicle_id, "Reset running passenger count");
            self.update_running_count(event, &mut state, vehicle_id, true);
        } else {
            self.update_running_count(event, &mut state, vehicle_id, false);
        }

        self.update_occupancy(&mut state, vehicle_id, seating_capacity, total_capacity);

        let state_json =
            serde_json::to_string(&state).map_err(|err| Error::State(err.to_string()))?;
        let last_value =
            self.store.replace_with_ttl(&state_key, state_json.as_bytes(), self.config.apc_ttl)?;
        if let (Some(before), Some(during)) = (state_prev.as_ref(), last_value.as_ref()) {
            if before != during {
                warn!(
                    vehicle_id = %vehicle_id,
                    previous = %String::from_utf8_lossy(before),
                    replaced = %String::from_utf8_lossy(during),
                    "State overwritten concurrently"
                );
            }
        }

        if let Some(ref occupancy) = state.occupancy_status {
            let occupancy_key = format!("{}:{}", self.config.redis.key_occupancy, vehicle_id);
            self.store.set_string_with_ttl(
                &occupancy_key,
                occupancy,
                self.config.occupancy_state_ttl,
            )?;
        }

        let count_key = format!("{}:{}", self.config.redis.apc_vehicle_id_key, vehicle_id);
        self.store.set_string_with_ttl(
            &count_key,
            &state.count.to_string(),
            self.config.apc_ttl,
        )?;

        Ok(())
    }

    fn migrate_legacy_keys(&self, vehicle_id: &str, state: &mut DilaxState) -> Result<()> {
        let migration_key =
            format!("{}:{}", self.config.redis.apc_vehicle_id_migrated_key, vehicle_id);
        if self.store.get_string(&migration_key)?.is_some() {
            return Ok(());
        }

        let legacy_trip_key = format!("{}:{}", self.config.redis.apc_vehicle_trip_key, vehicle_id);
        if let Some(trip_id) = self.store.get_string(&legacy_trip_key)? {
            warn!(vehicle_id = %vehicle_id, trip_id = %trip_id, "Migrating legacy trip ID");
            state.last_trip_id = Some(trip_id);
        }

        let legacy_count_key = format!("{}:{}", self.config.redis.apc_vehicle_id_key, vehicle_id);
        if let Some(count) = self.store.get_string(&legacy_count_key)? {
            if let Ok(count_int) = count.parse::<i64>() {
                warn!(vehicle_id = %vehicle_id, count = count_int, "Migrating legacy passenger count");
                state.count = count_int;
            }
        }

        self.store.set_string(&migration_key, "true")?;
        Ok(())
    }

    fn update_running_count(
        &self, event: &DilaxEvent, state: &mut DilaxState, vehicle_id: &str, skip_out: bool,
    ) {
        let mut total_in = 0_i64;
        let mut total_out = 0_i64;
        let mut total_out_no_skip = 0_i64;
        for door in &event.doors {
            total_in += i64::from(door.passengers_in);
            total_out_no_skip += i64::from(door.passengers_out);
            if !skip_out {
                total_out += i64::from(door.passengers_out);
            }
        }

        info!(
            vehicle_id = %vehicle_id,
            total_in,
            total_out,
            total_out_no_skip,
            skip_out,
            "Accumulated door counts"
        );

        let previous = state.count;
        let current = (previous - total_out).max(0) + total_in;
        if current < 0 {
            warn!(vehicle_id = %vehicle_id, count = current, "Calculated negative passenger count");
        }
        state.count = current.max(0);
        info!(vehicle_id = %vehicle_id, passenger_count = state.count, "Updated running passenger count");
    }

    fn update_occupancy(
        &self, state: &mut DilaxState, vehicle_id: &str, seating_capacity: i64, total_capacity: i64,
    ) {
        let occupancy = if state.count < (seating_capacity as f64 * 0.05).trunc() as i64 {
            OccupancyStatus::Empty
        } else if state.count < (seating_capacity as f64 * 0.4).trunc() as i64 {
            OccupancyStatus::ManySeatsAvailable
        } else if state.count < (seating_capacity as f64 * 0.9).trunc() as i64 {
            OccupancyStatus::FewSeatsAvailable
        } else if state.count < (total_capacity as f64 * 0.9).trunc() as i64 {
            OccupancyStatus::StandingRoomOnly
        } else {
            OccupancyStatus::Full
        };

        info!(vehicle_id = %vehicle_id, occupancy = %occupancy, "Updated occupancy status");
        state.occupancy_status = Some(occupancy.to_string());
    }

    async fn save_vehicle_trip_info(
        &self, vehicle_id: &str, vehicle_label: Option<&str>, trip_id: Option<String>,
        stop_id: Option<String>, event: &DilaxEvent,
    ) -> Result<()> {
        let key = format!("{}:{}", self.config.redis.key_vehicle_trip_info, vehicle_id);
        let payload = VehicleTripInfo {
            vehicle_info: crate::types::VehicleInfo {
                vehicle_id: vehicle_id.to_string(),
                label: vehicle_label.map(ToOwned::to_owned),
            },
            trip_id,
            stop_id,
            last_received_timestamp: Some(event.clock.utc.clone()),
            dilax_message: Some(event.clone()),
        };
        self.store.set_json_with_ttl(&key, &payload, VEHICLE_TRIP_INFO_TTL)?;
        Ok(())
    }
}
