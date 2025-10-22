use std::sync::Arc;

use anyhow::Result;
use tracing::warn;

use crate::cache::CacheRepository;
use crate::config::Config;
use crate::data_access::{BlockAccess, FleetAccess, TripAccess};
use crate::god_mode::GodMode;
use crate::locks::KeyLocker;
use crate::model::events::{EventType, PassengerCountEvent, SmartrakEvent};
use crate::processor::{LocationProcessor, PassengerCountProcessor, SerialDataProcessor};
use crate::provider::AdapterProvider;

#[derive(Debug, Clone)]
pub struct Processor<P: AdapterProvider> {
    config: Arc<Config>,
    cache: Arc<CacheRepository<P::Cache>>,
    fleet_access: FleetAccess<P>,
    trip_access: TripAccess<P>,
    block_access: BlockAccess<P>,
    location_processor: LocationProcessor<P>,
    passenger_processor: PassengerCountProcessor<P>,
    serial_processor: SerialDataProcessor<P>,
    locker: KeyLocker,
    god_mode: Option<GodMode>,
}

#[derive(Debug, Clone)]
pub enum ProducedMessage {
    VehiclePosition { topic: String, payload: String, key: String },
    DeadReckoning { topic: String, payload: String, key: String },
}

impl<P: AdapterProvider> Processor<P> {
    pub fn new(config: Arc<Config>, provider: P, god_mode: Option<GodMode>) -> Self {
        let cache = Arc::new(CacheRepository::new(provider.cache_store()));
        let fleet_access =
            FleetAccess::new(Arc::clone(&config), provider.clone(), Arc::clone(&cache));
        let trip_access =
            TripAccess::new(Arc::clone(&config), provider.clone(), Arc::clone(&cache));
        let block_access =
            BlockAccess::new(Arc::clone(&config), provider.clone(), Arc::clone(&cache));
        let location_processor = LocationProcessor::new(
            Arc::clone(&config),
            Arc::clone(&cache),
            fleet_access.clone(),
            trip_access.clone(),
            block_access.clone(),
        );
        let passenger_processor =
            PassengerCountProcessor::new(Arc::clone(&config), Arc::clone(&cache));
        let serial_processor =
            SerialDataProcessor::new(Arc::clone(&config), trip_access.clone(), Arc::clone(&cache));

        Self {
            config,
            cache,
            fleet_access,
            trip_access,
            block_access,
            location_processor,
            passenger_processor,
            serial_processor,
            locker: KeyLocker::new(),
            god_mode,
        }
    }

    pub async fn process(
        &self, topic: &str, event: &mut SmartrakEvent,
    ) -> Result<Vec<ProducedMessage>> {
        if let Some(god_mode) = &self.god_mode {
            god_mode.preprocess(event);
        }

        match event.event_type {
            EventType::SerialData => {
                self.serial_processor.process(event).await?;
                Ok(vec![])
            }
            EventType::Location => {
                let vehicle_id_or_label =
                    event.vehicle_id_or_label().unwrap_or_default().to_string();
                let guard = self.locker.lock(format!("location:{vehicle_id_or_label}")).await;
                let result =
                    self.location_processor.process(topic, event, &vehicle_id_or_label).await;
                drop(guard);
                result
            }
            EventType::Unknown => {
                warn!(topic = topic, "ignored unknown event type");
                Ok(vec![])
            }
        }
    }

    pub async fn process_passenger_event(&self, event: PassengerCountEvent) -> Result<()> {
        self.passenger_processor.process(event).await
    }

    pub fn cache(&self) -> &Arc<CacheRepository<P::Cache>> {
        &self.cache
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
