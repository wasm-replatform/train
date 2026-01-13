#![cfg(target_arch = "wasm32")]

use dilax_adapter::{DetectionReply, DetectionRequest, DilaxMessage};
use dilax_apc_connector::{DilaxReply, DilaxRequest};
use r9k_adapter::R9kMessage;
use r9k_connector::{R9kReply, R9kRequest};
use smartrak_gtfs::{
    CafAvlMessage, PassengerCountMessage, ResetReply, ResetRequest, SetTripReply, SetTripRequest,
    SmarTrakMessage, TrainAvlMessage, VehicleInfoReply, VehicleInfoRequest,
};
use qwasr_sdk::{Config, HttpRequest, Identity, Publisher, StateStore, ensure_env};

qwasr_sdk::guest!({
    owner: "at",
    provider: Provider,
    http: [
        "/api/apc": post(DilaxRequest with_body, DilaxReply),
        "/inbound/xml": post(R9kRequest with_body, R9kReply),
        "/jobs/detector": get(DetectionRequest, DetectionReply),
        "/info/{vehicle_id}": get(VehicleInfoRequest, VehicleInfoReply),
        "/god-mode/set-trip/{vehicle_id}/{trip_id}": get(SetTripRequest, SetTripReply),
        "/god-mode/reset/{vehicle_id}": get(ResetRequest, ResetReply),
    ],
    messaging: [
        "realtime-r9k.v1": R9kMessage,
        "realtime-r9k-to-smartrak.v1": SmarTrakMessage,
        "realtime-dilax-apc.v2": DilaxMessage,
        "realtime-caf-avl.v1": CafAvlMessage,
        "realtime-train-avl.v1": TrainAvlMessage,
        "realtime-passenger-count.v1": PassengerCountMessage,
    ]
});

#[derive(Clone, Default)]
pub struct Provider;

impl Provider {
    #[must_use]
    pub fn new() -> Self {
        ensure_env!(
            "BLOCK_MGT_URL",
            "CC_STATIC_URL",
            "FLEET_URL",
            "GTFS_STATIC_URL",
            "TRIP_MANAGEMENT_URL",
            "AZURE_IDENTITY",
        );
        Self
    }
}

impl Config for Provider {}
impl HttpRequest for Provider {}
impl Identity for Provider {}
impl Publisher for Provider {}
impl StateStore for Provider {}
