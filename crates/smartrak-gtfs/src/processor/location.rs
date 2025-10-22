use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde_json;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::cache::CacheRepository;
use crate::config::{CACHE_TTL_SIGN_ON, CACHE_TTL_TRIP_TRAIN, Config};
use crate::data_access::{BlockAccess, FleetAccess, TripAccess, parse_trip_time};
use crate::model::dead_reckoning::{DeadReckoningMessage, PositionDr, VehicleDr};
use crate::model::events::{LocationData, SmartrakEvent};
use crate::model::fleet::VehicleInfo;
use crate::model::gtfs::{
    FeedEntity, OccupancyStatus, Position as GtfsPosition, TripDescriptorPayload,
    VehicleDescriptor, VehiclePosition,
};
use crate::model::trip::{BlockInstance, TripDescriptor, TripInstance};
use crate::processor::passenger_count::PassengerCountProcessor;
use crate::provider::AdapterProvider;
use crate::service::ProducedMessage;

#[derive(Debug, Clone)]
pub struct LocationProcessor<P: AdapterProvider> {
    config: Arc<Config>,
    cache: Arc<CacheRepository<P::Cache>>,
    fleet_access: FleetAccess<P>,
    trip_access: TripAccess<P>,
    block_access: BlockAccess<P>,
}

impl<P: AdapterProvider> LocationProcessor<P> {
    pub fn new(
        config: Arc<Config>, cache: Arc<CacheRepository<P::Cache>>, fleet_access: FleetAccess<P>,
        trip_access: TripAccess<P>, block_access: BlockAccess<P>,
    ) -> Self {
        Self { config, cache, fleet_access, trip_access, block_access }
    }

    pub async fn process(
        &self, topic: &str, event: &SmartrakEvent, vehicle_id_or_label: &str,
    ) -> Result<Vec<ProducedMessage>> {
        debug!(topic, event = ?event, "processing location event");
        if !self.is_valid(event) {
            warn!("invalid location event");
            return Ok(vec![]);
        }

        let Some(vehicle_info) = self
            .fleet_access
            .by_id_or_label(vehicle_id_or_label)
            .await
            .context("fetching vehicle information")?
        else {
            info!(vehicle = vehicle_id_or_label, "vehicle not found, skipping");
            return Ok(vec![]);
        };

        if topic.contains("caf-avl") {
            if !vehicle_info.matches_tag(crate::model::fleet::Tags::CAF) {
                info!(vehicle = vehicle_info.id, "CAF tag mismatch, skipping");
                return Ok(vec![]);
            }
        } else if !vehicle_info.matches_tag(crate::model::fleet::Tags::SMARTRAK)
            && !topic.contains("r9k-to-smartrak")
        {
            info!(vehicle = vehicle_info.id, "Smartrak tag mismatch");
            return Ok(vec![]);
        }

        self.process_event(topic, event, vehicle_info).await
    }

    async fn process_event(
        &self, topic: &str, event: &SmartrakEvent, vehicle: VehicleInfo,
    ) -> Result<Vec<ProducedMessage>> {
        let mut outputs = Vec::new();
        let event_timestamp = event.message_data.timestamp.unwrap_or_else(Utc::now);
        let event_secs = event_timestamp.timestamp();

        if vehicle.is_train() {
            if let Some(block_instance) = self
                .block_access
                .allocation(&vehicle.id, event_secs)
                .await
                .context("fetching block allocation")?
            {
                if block_instance.has_error() {
                    info!(vehicle = vehicle.id, topic, "block allocation error sentinel");
                } else {
                    self.assign_train_to_trip(&vehicle, event_secs, block_instance).await?;
                }
            }
        }

        let trip_descriptor = self
            .cached_trip_instance(&vehicle.id, event_secs)
            .await?
            .map(|trip| trip.to_trip_descriptor());

        if !event.location_data.has_coordinates() {
            if let Some(odometer) = event.location_data.odometer.or(event.event_data.odometer) {
                if let Some(trip) = trip_descriptor.clone() {
                    let dr = DeadReckoningMessage {
                        id: Uuid::new_v4().to_string(),
                        received_at: event_secs,
                        position: PositionDr { odometer },
                        trip,
                        vehicle: VehicleDr { id: vehicle.id.clone() },
                    };
                    outputs.push(ProducedMessage::DeadReckoning {
                        topic: self.config.topics.dr_topic.clone(),
                        key: vehicle.id.clone(),
                        payload: serde_json::to_string(&dr)?,
                    });
                }
            }
            return Ok(outputs);
        }

        let entity =
            self.build_feed_entity(event, &vehicle, trip_descriptor.as_ref(), event_secs).await?;
        outputs.push(ProducedMessage::VehiclePosition {
            topic: self.config.topics.vp_topic.clone(),
            key: entity.id.clone(),
            payload: serde_json::to_string(&entity)?,
        });
        Ok(outputs)
    }

    fn is_valid(&self, event: &SmartrakEvent) -> bool {
        if event.remote_data.external_id.is_none() {
            warn!("missing remote data");
            return false;
        }

        if let Some(acc) = event.location_data.gps_accuracy {
            if acc < self.config.accuracy_threshold {
                info!(
                    accuracy = acc,
                    threshold = self.config.accuracy_threshold,
                    "rejecting low accuracy"
                );
                return false;
            }
        }
        true
    }

    async fn assign_train_to_trip(
        &self, vehicle: &VehicleInfo, event_timestamp: i64, block_instance: BlockInstance,
    ) -> Result<()> {
        let trip_key = self.config.trip_key(&vehicle.id);
        let sign_on_key = self.config.sign_on_key(&vehicle.id);

        if block_instance.vehicle_ids.first().map(|id| id != &vehicle.id).unwrap_or(true) {
            self.cache.delete(&sign_on_key).await?;
            self.cache.delete(&trip_key).await?;
            return Ok(());
        }

        if block_instance.trip_id.is_empty() {
            return Ok(());
        }

        if let Some(prev) = self.cache.get_json::<TripInstance>(&trip_key).await? {
            if prev.trip_id == block_instance.trip_id
                && prev.start_time() == Some(block_instance.start_time.as_str())
                && prev.service_date() == Some(block_instance.service_date.as_str())
            {
                return Ok(());
            }
        }

        let new_trip = self
            .trip_access
            .get_trip_instance(
                &block_instance.trip_id,
                &block_instance.service_date,
                &block_instance.start_time,
            )
            .await?
            .filter(|trip| !trip.has_error());

        match new_trip {
            Some(trip) => {
                self.cache
                    .set_ex(&sign_on_key, CACHE_TTL_SIGN_ON, event_timestamp.to_string())
                    .await?;
                self.cache.set_json_ex(&trip_key, CACHE_TTL_TRIP_TRAIN, &trip).await?;
            }
            None => {
                self.cache.delete(&sign_on_key).await?;
                self.cache.delete(&trip_key).await?;
            }
        }

        Ok(())
    }

    async fn cached_trip_instance(
        &self, vehicle_id: &str, event_timestamp: i64,
    ) -> Result<Option<TripInstance>> {
        let trip_key = self.config.trip_key(vehicle_id);
        let Some(trip) = self.cache.get_json::<TripInstance>(&trip_key).await? else {
            return Ok(None);
        };

        if trip.has_error() {
            return Ok(None);
        }

        if let (Some(start_time), Some(end_time), Some(service_date)) =
            (trip.start_time(), trip.end_time(), trip.service_date())
        {
            let sign_on_key = self.config.sign_on_key(vehicle_id);
            if let Some(sign_on_raw) = self.cache.get(&sign_on_key).await? {
                let sign_on_secs = sign_on_raw.parse::<i64>().ok();
                if let (Some(sign_on_secs), Some(start_unix), Some(end_unix)) = (
                    sign_on_secs,
                    parse_trip_time(self.config.timezone, service_date, start_time),
                    parse_trip_time(self.config.timezone, service_date, end_time),
                ) {
                    let duration = end_unix - start_unix + self.config.trip_duration_buffer;
                    if event_timestamp - duration > sign_on_secs {
                        info!(vehicle = vehicle_id, "event beyond trip duration window");
                        return Ok(None);
                    }
                }
            }
        }

        Ok(Some(trip))
    }

    async fn build_feed_entity(
        &self, event: &SmartrakEvent, vehicle: &VehicleInfo,
        trip_descriptor: Option<&TripDescriptor>, event_secs: i64,
    ) -> Result<FeedEntity> {
        let occupancy_status = match trip_descriptor {
            Some(trip) => self.occupancy_status(&vehicle.id, trip).await?,
            None => None,
        };

        let position =
            Self::position(&event.location_data).ok_or_else(|| anyhow!("missing coordinates"))?;

        let vehicle_position = VehiclePosition {
            position: Some(position),
            trip: trip_descriptor.map(TripDescriptorPayload::from),
            vehicle: Self::vehicle_descriptor(vehicle),
            occupancy_status,
            timestamp: event_secs,
        };

        Ok(FeedEntity { id: vehicle.id.clone(), vehicle: vehicle_position })
    }

    async fn occupancy_status(
        &self, vehicle_id: &str, trip: &TripDescriptor,
    ) -> Result<Option<OccupancyStatus>> {
        PassengerCountProcessor::<P>::lookup_occupancy(
            self.cache.as_ref(),
            self.config.as_ref(),
            vehicle_id,
            trip,
        )
        .await
    }

    fn vehicle_descriptor(vehicle: &VehicleInfo) -> VehicleDescriptor {
        VehicleDescriptor {
            id: vehicle.id.clone(),
            label: vehicle.label.clone(),
            license_plate: vehicle.registration.clone(),
        }
    }

    fn position(location: &LocationData) -> Option<GtfsPosition> {
        let latitude = location.latitude?;
        let longitude = location.longitude?;
        Some(GtfsPosition {
            latitude,
            longitude,
            bearing: location.heading,
            speed: location.speed.map(|speed| (speed * 1000.0) / 3600.0),
            odometer: location.odometer,
        })
    }
}
