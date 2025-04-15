use crate::{error::Result, preset::Moment};
use config::{Environment, File};
use serde::Deserialize;
use std::collections::BTreeMap;
use tracing::debug;

/// Application configuration sourced from env and/or config file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<i32>,
    #[serde(default)]
    pub disable_night_mode: bool,
    #[serde(default)]
    pub presets: BTreeMap<String, PresetConfig>,
}

/// Configuration of a time-based preset.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PresetConfig {
    pub enabled: bool,
    pub from: Moment,
    pub to: Moment,
    pub wreck_havoc: Option<bool>,
    pub esp_deep_sleep_seconds: Option<u64>,
}

impl Config {
    /// Loads the application configuration config files and environment variables.
    pub fn load() -> Result<Config> {
        let config = config::Config::builder()
            // Configuration from `config.toml`.
            .add_source(File::with_name("config").required(false))
            // Config from environment variables.
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()?;

        debug!(?config, "configuration loaded");

        Ok(config)
    }
}
