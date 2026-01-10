mod provider;

use dilax_apc_connector::{DilaxMessage, DilaxRequest};
use warp_sdk::Handler;

use self::provider::MockProvider;

#[tokio::test]
async fn device_site_header() {
    let provider = MockProvider::default();
    let payload = include_bytes!("../data/dilax-message.json");

    DilaxRequest::handler(payload.to_vec())
        .expect("should deserialize")
        .provider(&provider)
        .owner("owner")
        .await
        .expect("should succeed");

    let published = provider.published();
    assert_eq!(published.len(), 1);

    let (topic, record) = &published[0];
    assert_eq!(topic, "dev-realtime-dilax-apc.v2");

    let message: DilaxMessage = serde_json::from_slice(payload).expect("should deserialize");

    let expected_key = message.device.as_ref().expect("device").site.as_str();
    assert_eq!(record.headers.get("key").map(String::as_str), Some(expected_key));

    let expected_payload = serde_json::to_vec(&message).expect("should serialize");
    assert_eq!(record.payload, expected_payload);
}

#[tokio::test]
async fn device_missing() {
    let provider = MockProvider::default();
    let payload = include_bytes!("../data/dilax-no-device.json");

    DilaxRequest::handler(payload.to_vec())
        .expect("should deserialize")
        .provider(&provider)
        .owner("owner")
        .await
        .expect("should succeed");

    let published = provider.published();
    assert_eq!(published.len(), 1);

    let (topic, record) = &published[0];
    assert_eq!(topic, "dev-realtime-dilax-apc.v2");
    assert_eq!(record.headers.get("key").map(String::as_str), Some("undefined"));

    let message: DilaxMessage = serde_json::from_slice(payload).expect("should deserialize");
    let expected_payload = serde_json::to_vec(&message).expect("should serialize");
    assert_eq!(record.payload, expected_payload);
}

#[tokio::test]
async fn device_site_whitespace() {
    let provider = MockProvider::default();
    let payload = include_bytes!("../data/dilax-whitespace.json");

    DilaxRequest::handler(payload.to_vec())
        .expect("should deserialize")
        .provider(&provider)
        .owner("owner")
        .await
        .expect("should succeed");

    let published = provider.published();
    assert_eq!(published.len(), 1);

    let (topic, record) = &published[0];
    assert_eq!(topic, "dev-realtime-dilax-apc.v2");
    assert_eq!(record.headers.get("key").map(String::as_str), Some("  "));

    let message: DilaxMessage = serde_json::from_slice(payload).expect("should deserialize");
    let expected_payload = serde_json::to_vec(&message).expect("should serialize");
    assert_eq!(record.payload, expected_payload);
}
