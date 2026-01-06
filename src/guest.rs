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
        "/jobs/detector": {
            method: get,
            request: DetectionRequest,
            reply: DetectionReply
        },
        "/inbound/xml": {
            method: post,
            request: R9kRequest,
            reply: R9kReply
        },
        "/god-mode/set-trip/{vehicle_id}/{trip_id}": {
            method: get,
            request: SetTripRequest,
            reply: SetTripReply,
        },
        "/god-mode/reset/{vehicle_id}": {
            method: get,
            request: VehicleInfoRequest,
            reply: VehicleInfoReply
        }
    ],
    messaging: [
        "realtime-r9k.v1": {
            message: R9kMessage
        }
    ]
});
