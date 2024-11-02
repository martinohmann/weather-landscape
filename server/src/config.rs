use crate::error::Result;
use config::{Environment, File};
use serde::Deserialize;
use tracing::debug;

/// Application configuration sourced from env and/or config file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub image: ImageConfig,
    pub weather: WeatherConfig,
}

/// Image configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ImageConfig {
    /// Disable inverting colors at night time.
    #[serde(default)]
    pub disable_night_mode: bool,
}

/// Weather configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherConfig {
    /// The latitude of the location.
    pub latitude: f64,
    /// The longitude of the location.
    pub longitude: f64,
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
