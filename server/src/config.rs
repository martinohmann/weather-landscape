use crate::error::Result;
use config::{builder::AsyncState, ConfigBuilder, Environment, File};
use serde::Deserialize;
use tracing::debug;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub disable_night_mode: bool,
}

impl Config {
    /// Loads the application configuration config files and environment variables.
    pub async fn load() -> Result<Config> {
        let config = ConfigBuilder::<AsyncState>::default()
            // Configuration from `config.toml`.
            .add_source(File::with_name("config").required(false))
            // Config from environment variables.
            .add_source(Environment::default().separator("_"))
            .build()
            .await?
            .try_deserialize()?;

        debug!(?config, "configuration loaded");

        Ok(config)
    }
}
