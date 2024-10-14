use crate::error::Result;
use config::{builder::AsyncState, ConfigBuilder, Environment, File};
use log::debug;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub latitude: f64,
    pub longitude: f64,
}

impl AppConfig {
    /// Loads the application configuration config files and environment variables.
    pub async fn load() -> Result<AppConfig> {
        let config = ConfigBuilder::<AsyncState>::default()
            // Configuration from `config.toml`.
            .add_source(File::with_name("config").required(false))
            // Config from environment variables.
            .add_source(Environment::default().separator("_"))
            .build()
            .await?
            .try_deserialize()?;

        debug!("loaded configuration: {:?}", config);

        Ok(config)
    }
}
