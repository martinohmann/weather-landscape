use crate::error::Result;
use config::{Environment, File};
use serde::Deserialize;
use tracing::debug;

/// Application configuration sourced from env and/or config file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub disable_night_mode: bool,
}

impl Config {
    /// Loads the application configuration config files and environment variables.
    pub fn load() -> Result<Config> {
        let config = config::Config::builder()
            // Configuration from `config.toml`.
            .add_source(File::with_name("config").required(false))
            // Config from environment variables.
            .add_source(Environment::default().separator("_"))
            .build()?
            .try_deserialize()?;

        debug!(?config, "configuration loaded");

        Ok(config)
    }
}
