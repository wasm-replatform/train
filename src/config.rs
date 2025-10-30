pub fn get_gtfs_cc_static_url() -> String {
    std::env::var("GTFS_CC_STATIC_URL").unwrap_or_else(|_| {
        let default = "https://www-dev-cc-static-api-01.azurewebsites.net".to_string();
        tracing::trace!("GTFS_CC_STATIC_URL not set, using default: {default}");
        default
    })
}

pub fn get_block_mgt_url() -> String {
    std::env::var("BLOCK_MANAGEMENT_URL").unwrap_or_else(|_| {
        let default = "https://www-dev-block-mgt-client-api-01.azurewebsites.net".to_string();
        tracing::trace!("BLOCK_MANAGEMENT_URL not set, using default: {default}");
        default
    })
}

pub fn get_block_mgt_bearer_token() -> Option<String> {
    const CANDIDATES: [&str; 3] =
        ["BLOCK_MGT_BEARER_TOKEN", "BLOCK_MANAGEMENT_BEARER_TOKEN", "BLOCK_MGT_CLIENT_API_BEARER"];

    for key in CANDIDATES {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                tracing::trace!("{key} loaded from environment");
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

pub fn get_fleet_api_url() -> String {
    std::env::var("FLEET_API_URL").unwrap_or_else(|_| {
        let default = "https://www-dev-fleet-api-01.azurewebsites.net".to_string();
        tracing::trace!("FLEET_API_URL not set, using default: {default}");
        default
    })
}

pub fn get_gtfs_static_url() -> String {
    std::env::var("GTFS_STATIC_URL").unwrap_or_else(|_| {
        let default = "https://www-dev-gtfs-static-api-01.azurewebsites.net".to_string();
        tracing::trace!("GTFS_STATIC_URL not set, using default: {default}");
        default
    })
}

pub fn get_r9k_source_topic() -> String {
    std::env::var("KAFKA_SOURCE_TOPIC").unwrap_or_else(|_| {
        let default = "dev-realtime-r9k.v1".to_string();
        tracing::trace!("KAFKA_SOURCE_TOPIC not set, using default: {default}");
        default
    })
}

pub fn get_dilax_source_topic() -> String {
    std::env::var("KAFKA_DILAX_SOURCE_TOPIC").unwrap_or_else(|_| {
        let default = "dev-realtime-dilax-apc.v1".to_string();
        tracing::trace!("KAFKA_DILAX_SOURCE_TOPIC not set, using default: {default}");
        default
    })
}

pub fn get_dilax_outbound_topic() -> String {
    std::env::var("KAFKA_DILAX_OUTBOUND_TOPIC").unwrap_or_else(|_| {
        let default = "dev-realtime-smartrak-train-avl.v1".to_string();
        tracing::trace!("KAFKA_DILAX_OUTBOUND_TOPIC not set, using default: {default}");
        default
    })
}
