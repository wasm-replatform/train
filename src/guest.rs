#![cfg(target_arch = "wasm32")]

mod provider;

use dilax_adapter::{DetectionReply, DetectionRequest};
use r9k_adapter::R9kMessage;
use r9k_connector::{R9kReply, R9kRequest};
use smartrak_gtfs::{SetTripReply, SetTripRequest, VehicleInfoReply, VehicleInfoRequest};

use crate::provider::Provider;

warp_sdk::guest!({
    owner: "at",
    provider: Provider,
    http: [
        "/jobs/detector": get(DetectionRequest, DetectionReply),
        "/inbound/xml": post(R9kRequest with_body, R9kReply),
        "/god-mode/set-trip/{vehicle_id}/{trip_id}": get(SetTripRequest, SetTripReply),
        "/god-mode/reset/{vehicle_id}": get(VehicleInfoRequest, VehicleInfoReply),
    ],
    messaging: [
        "realtime-r9k.v1": R9kMessage,
    ]
});
