use crate::{config::Config, error::Result, weather::Weather};
use prometheus::{opts, IntCounterVec, Registry};

/// Holds the application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub metrics: Metrics,
    pub weather: Weather,
}

impl AppState {
    /// Creates `AppState` from config and metrics.
    pub fn new(config: Config, metrics: Metrics) -> Result<AppState> {
        let weather = Weather::new(config.latitude, config.longitude)?;

        Ok(AppState {
            config,
            metrics,
            weather,
        })
    }
}

/// Container type for all custom application metrics.
#[derive(Clone, Debug)]
pub struct Metrics {
    /// Counter for the total number of image requests.
    pub image_counter: IntCounterVec,
}

impl Metrics {
    /// Creates metrics using the given namespace and registers them to the prometheus registry.
    pub fn new(namespace: &str, registry: &Registry) -> Result<Metrics> {
        let image_counter = IntCounterVec::new(
            opts!("image_requests_total", "Total number of image requests").namespace(namespace),
            &["mime_type"],
        )?;

        registry.register(Box::new(image_counter.clone()))?;

        Ok(Metrics { image_counter })
    }
}
