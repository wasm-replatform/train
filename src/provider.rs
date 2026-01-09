use anyhow::Result;
use fromenv::FromEnv;
use warp_sdk::{Config, HttpRequest, Identity, Publisher, StateStore};

#[derive(Clone)]
pub struct Provider {
    pub config: ConfigSettings,
}

impl Provider {
    pub fn new() -> Self {
        Self { config: ConfigSettings::default() }
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConfigSettings {
    #[env(from = "ENV", default = "dev")]
    pub environment: String,
    #[env(from = "BLOCK_MGT_URL")]
    pub block_mgt_url: String,
    #[env(from = "CC_STATIC_URL")]
    pub cc_static_url: String,
    #[env(from = "FLEET_URL")]
    pub fleet_url: String,
    #[env(from = "GTFS_STATIC_URL")]
    pub gtfs_static_url: String,
    #[env(from = "AZURE_IDENTITY")]
    pub azure_identity: String,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        // We panic here to ensure configuration is always loaded.
        // i.e. the guest should not start without proper configuration.
        Self::from_env().finalize().expect("should load configuration")
    }
}

impl Config for Provider {
    async fn get(&self, key: &str) -> Result<String> {
        Ok(match key {
            "ENV" => &self.config.environment,
            "BLOCK_MGT_URL" => &self.config.block_mgt_url,
            "CC_STATIC_URL" => &self.config.cc_static_url,
            "FLEET_URL" => &self.config.fleet_url,
            "GTFS_STATIC_URL" => &self.config.gtfs_static_url,
            "AZURE_IDENTITY" => &self.config.azure_identity,
            _ => return Err(anyhow::anyhow!("unknown config key: {key}")),
        }
        .clone())
    }
}

// Use default implementations for these traits
impl HttpRequest for Provider {}
impl Identity for Provider {}
impl Publisher for Provider {}
impl StateStore for Provider {}
