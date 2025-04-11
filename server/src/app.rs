use crate::{config::Config, error::Result, graphics::Renderer, weather::Weather};
use prometheus::{
    IntCounterVec, Registry,
    core::{AtomicU64, GenericCounter},
    opts,
};

/// Holds the application state.
#[derive(Clone)]
pub struct AppState {
    pub metrics: Metrics,
    pub renderer: Renderer,
    pub weather: Weather,
}

impl AppState {
    /// Creates `AppState` from config and metrics.
    pub fn new(config: &Config, metrics: Metrics) -> Result<AppState> {
        let weather = Weather::new(config.latitude, config.longitude, config.altitude)?;
        let renderer = Renderer::new(config, metrics.clone());

        Ok(AppState {
            metrics,
            renderer,
            weather,
        })
    }
}

/// Container type for all custom application metrics.
#[derive(Clone, Debug)]
pub struct Metrics {
    image_counter: IntCounterVec,
    object_counter: IntCounterVec,
}

impl Metrics {
    /// Creates metrics using the given namespace and registers them to the prometheus registry.
    pub fn new(namespace: &str, registry: &Registry) -> Result<Metrics> {
        let image_counter = IntCounterVec::new(
            opts!("image_requests_total", "Total number of image requests").namespace(namespace),
            &["mime_type"],
        )?;
        let object_counter = IntCounterVec::new(
            opts!("rendered_objects_total", "Total number of rendered objects")
                .namespace(namespace),
            &["object"],
        )?;

        registry.register(Box::new(image_counter.clone()))?;
        registry.register(Box::new(object_counter.clone()))?;

        Ok(Metrics {
            image_counter,
            object_counter,
        })
    }

    /// Returns the image counter for given mime type.
    pub fn image_counter(&self, mime_type: &str) -> GenericCounter<AtomicU64> {
        self.image_counter.with_label_values(&[mime_type])
    }

    /// Returns the counter for given object.
    pub fn object_counter(&self, object: &str) -> GenericCounter<AtomicU64> {
        self.object_counter.with_label_values(&[object])
    }
}
