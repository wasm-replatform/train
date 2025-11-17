use chrono::{Duration, Utc};
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::models::{DecodedSerialData, SmartrakEvent, TripInstance};
use crate::provider::{Provider, StateStore};
use crate::trip;

const TTL_TRIP_SERIAL: Duration = Duration::seconds(4 * 60 * 60);
const TTL_SIGN_ON: Duration = Duration::seconds(24 * 60 * 60);

pub async fn process_serial_data(provider: &impl Provider, event: &SmartrakEvent) -> Result<()> {
    if !is_serial_event_valid(event) {
        return Ok(());
    }

    let Some(remote) = event.remote_data.as_ref() else {
        return Err(Error::MissingField("remoteData"));
    };
    let Some(vehicle_id) = remote.external_id.as_deref() else {
        return Err(Error::MissingField("remoteData.externalId"));
    };

    let event_timestamp = event
        .timestamp_unix()
        .ok_or_else(|| Error::InvalidTimestamp(event.message_data.timestamp.clone()))?;

    let Some(serial_data) = event.serial_data.as_ref() else {
        return Err(Error::MissingField("serialData"));
    };
    let Some(decoded) = serial_data.decoded_serial_data.as_ref() else {
        return Err(Error::MissingField("serialData.decodedSerialData"));
    };

    let lock_key = format!("serialData:{vehicle_id}");

    if is_old_event(vehicle_id, event_timestamp) {
        return Ok(());
    }

    allocate_vehicle_to_trip(vehicle_id, decoded, event_timestamp, provider).await
}

fn is_serial_event_valid(event: &SmartrakEvent) -> bool {
    let Some(remote) = event.remote_data.as_ref() else {
        return false;
    };
    let has_vehicle = remote.external_id.is_some();
    let has_serial =
        event.serial_data.as_ref().and_then(|serial| serial.decoded_serial_data.as_ref()).is_some();

    if !has_vehicle || !has_serial {
        return false;
    }

    let Some(timestamp) = event.timestamp_unix() else {
        return false;
    };

    let future_delta = timestamp - Utc::now().timestamp();

    let serial_data_filter_threshold =
            Duration::seconds(env_i64("SERIAL_DATA_FILTER_THRESHOLD", 900));
    if future_delta > serial_data_filter_threshold.num_seconds() {
        warn!(future_delta, "serial data event rejected because it is from the future");
        return false;
    }

    true
}

fn is_old_event(vehicle_id: &str, timestamp: i64) -> bool {
    if let Some(prev) = vehicle_timestamps.get(vehicle_id) {
        if *prev > timestamp {
            warn!(vehicle_id, "received older serial data event");
            return true;
        }
    }

    vehicle_timestamps.insert(vehicle_id.to_string(), timestamp);
    false
}

async fn allocate_vehicle_to_trip(
    vehicle_id: &str, decoded: &DecodedSerialData, event_timestamp: i64, provider: &impl Provider
) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle_id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle_id);

    let Some(trip_id) = decoded.trip_id.as_deref() else {
        debug!(vehicle_id, "serial data without trip id, clearing cache");
        StateStore::delete(provider, &sign_on_key).await?;
        StateStore::delete(provider, &trip_key).await?;
        return Ok(());
    };

    let bytes = StateStore::get(provider, &trip_key).await?;
    
    let previous: Option<TripInstance> = serde_json::from_slice(Some(&bytes)).context("deserializing vehicle trip info")?;

    if let Some(prev_trip) = previous {
        if prev_trip.trip_id == trip_id {
            return Ok(());
        }

        let new_trip =
            trip::get_nearest_trip_instance(provider, trip_id, event_timestamp)
                .await?;

        if new_trip.as_ref().is_none_or(|trip| trip.has_error()) {
            StateStore::delete(provider, &sign_on_key).await?;
            StateStore::delete(provider, &trip_key).await?;
            return Ok(());
        }

        if let Some(trip) = new_trip {
            persist_trip(vehicle_id, event_timestamp, trip, provider).await?;
        }

        return Ok(());
    }

    let new_trip =
        trip::get_nearest_trip_instance(provider, trip_id, event_timestamp)
            .await?;

    if let Some(trip) = new_trip {
        if trip.has_error() {
            StateStore::delete(provider,&sign_on_key).await?;
            StateStore::delete(provider, &trip_key).await?;
            return Ok(());
        }

        persist_trip(vehicle_id, event_timestamp, trip, provider).await?;
    }

    Ok(())
}

async fn persist_trip(vehicle_id: &str, event_timestamp: i64, trip: TripInstance, provider: &impl Provider) -> Result<()> {
    let trip_key = format!("smartrakGtfs:trip:vehicle:{}", &vehicle_id);
    let sign_on_key = format!("smartrakGtfs:vehicle:signOn:{}", &vehicle_id);

    StateStore::set(provider, &trip_key, &trip, Some(TTL_TRIP_SERIAL)).await?;
    StateStore::set(provider, &sign_on_key, &event_timestamp, Some(TTL_SIGN_ON)).await
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::error::Error as StdError;

    use anyhow::{Result as AnyhowResult, anyhow};
    use bytes::Bytes;
    use dashmap::DashMap;
    use http::{Request, Response};

    use super::*;
    use crate::error::Result as SmartrakResult;
    use crate::provider::{HttpRequest, Identity, StateStore};

    #[derive(Clone, Default)]
    struct TestProvider;

    impl HttpRequest for TestProvider {
        fn fetch<T>(
            _request: Request<T>,
        ) -> impl std::future::Future<Output = AnyhowResult<Response<Bytes>>> + Send
        where
            T: http_body::Body + Any + Send,
            T::Data: Into<Vec<u8>>,
            T::Error: Into<Box<dyn StdError + Send + Sync + 'static>>,
        {
            async { Err(anyhow!("not implemented")) }
        }
    }

    impl StateStore for TestProvider {
        fn get(
            _key: &str,
        ) -> impl std::future::Future<Output = AnyhowResult<Option<Vec<u8>>>> + Send {
            async { Ok(None) }
        }

        fn set(
            _key: &str, _value: &[u8], _ttl_secs: Option<u64>,
        ) -> impl std::future::Future<Output = AnyhowResult<Option<Vec<u8>>>> + Send {
            async { Ok(None) }
        }

        fn delete(_key: &str) -> impl std::future::Future<Output = AnyhowResult<()>> + Send {
            async { Ok(()) }
        }
    }

    impl Identity for TestProvider {
        fn access_token(&self) -> impl std::future::Future<Output = AnyhowResult<String>> + Send {
            async { Ok(String::new()) }
        }
    }

    #[derive(Default)]
    struct MemoryCache(DashMap<String, Vec<u8>>);

    #[async_trait::async_trait]
    impl CacheStore for MemoryCache {
        async fn get_value(key: &str) -> SmartrakResult<Option<Vec<u8>>> {
            Ok(0.get(key).map(|value| value.value().clone()))
        }

        async fn set_value(key: &str, value: &[u8], _ttl: Option<Duration>) -> SmartrakResult<()> {
            0.insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn delete(key: &str) -> SmartrakResult<()> {
            0.remove(key);
            Ok(())
        }
    }
}
