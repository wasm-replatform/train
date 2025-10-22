use dashmap::DashMap;
use tracing::debug;

use crate::model::events::EventType;
use crate::model::events::SmartrakEvent;

#[derive(Debug, Default, Clone)]
pub struct GodMode {
    vehicle_to_trip: DashMap<String, String>,
}

impl GodMode {
    pub fn reset_all(&self) {
        self.vehicle_to_trip.clear();
    }

    pub fn reset_vehicle(&self, vehicle_id: &str) {
        self.vehicle_to_trip.remove(vehicle_id);
    }

    pub fn set_vehicle_to_trip(&self, vehicle_id: impl Into<String>, trip_id: impl Into<String>) {
        self.vehicle_to_trip.insert(vehicle_id.into(), trip_id.into());
    }

    pub fn describe(&self) -> String {
        let mut entries: Vec<_> = self
            .vehicle_to_trip
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        entries.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        serde_json::to_string(&entries).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn preprocess(&self, event: &mut SmartrakEvent) {
        if event.event_type != EventType::SerialData {
            return;
        }

        let Some(remote) = event.remote_data.external_id.clone() else {
            return;
        };

        if let Some(mapping_ref) = self.vehicle_to_trip.get(&remote) {
            let mapping = mapping_ref.value().clone();
            debug!(vehicle = %remote, trip = %mapping, "god mode override");
            let serial = event.serial_data.decoded_serial_data.get_or_insert_with(Default::default);
            if mapping.as_str() == "empty" {
                serial.line_id = Some(String::new());
                serial.trip_id = Some(String::new());
                serial.trip_number = Some(String::new());
            } else {
                serial.line_id = Some(String::new());
                serial.trip_id = Some(mapping.clone());
                serial.trip_number = Some(mapping);
            }
        }
    }
}
