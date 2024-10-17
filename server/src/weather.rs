use crate::error::{Error, Result};
use jiff::Timestamp;
use monsoon::{
    body::{Body, TimeSeries},
    Monsoon, Params, Response,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::{
    limit::{ConcurrencyLimit, RateLimit},
    Service, ServiceBuilder, ServiceExt,
};

// Met.no requires to identify oneself via user-agent header. This is best practice anyways.
const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    "(",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

#[derive(Debug)]
struct Inner {
    service: ConcurrencyLimit<RateLimit<Monsoon>>,
    last_response: Option<Response>,
    latitude: f64,
    longitude: f64,
}

impl Inner {
    async fn get(&mut self) -> Result<WeatherData> {
        let response = self
            .service
            .ready()
            .await?
            .call(Params::new_with_last_response(
                self.latitude,
                self.longitude,
                None,
                self.last_response.clone(),
            )?)
            .await?;

        self.last_response = Some(response.clone());
        let body = response.body()?;

        WeatherData::from_body(&body)
    }
}

#[derive(Debug, Clone)]
pub struct Weather {
    inner: Arc<Mutex<Inner>>,
}

impl Weather {
    /// Create a new weather service for the location at `latitude`/`longitude`.
    pub fn new(latitude: f64, longitude: f64) -> Result<Self> {
        let monsoon = Monsoon::new(USER_AGENT)?;

        // Limit request volume according to the met.no TOS: https://api.met.no/doc/TermsOfService.
        let service = ServiceBuilder::new()
            .concurrency_limit(10)
            .rate_limit(20, Duration::from_secs(1))
            .service(monsoon);

        Ok(Weather {
            inner: Arc::new(Mutex::new(Inner {
                service,
                last_response: None,
                latitude,
                longitude,
            })),
        })
    }

    /// Fetches weather data.
    pub async fn get(&self) -> Result<WeatherData> {
        self.inner.lock().await.get().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    pub coords: Coords,
    pub current: DataPoint,
    pub forecasts: Vec<DataPoint>,
}

impl WeatherData {
    fn from_body(body: &Body) -> Result<WeatherData> {
        let time_series = &body.properties.timeseries;

        if time_series.is_empty() {
            return Err(Error::new("empty time series"));
        }

        let current = DataPoint::from_time_series(&time_series[0])?;

        let forecasts = time_series
            .iter()
            .skip(1) // The current weather.
            .take(24) // 24 hours of forecast data.
            .map(DataPoint::from_time_series)
            .collect::<Result<Vec<_>>>()?;

        if forecasts.len() < 24 {
            return Err(Error::new("not enough forecast data"));
        }

        Ok(WeatherData {
            coords: Coords {
                latitude: body.geometry.coordinates.latitude,
                longitude: body.geometry.coordinates.longitude,
            },
            current,
            forecasts,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct DataPoint {
    pub air_temperature: f64,
    pub cloud_area_fraction: f64,
    pub precipitation_amount: f64,
    pub timestamp: Timestamp,
    pub wind_from_direction: f64,
    pub wind_speed: f64,
}

impl DataPoint {
    fn from_time_series(series: &TimeSeries) -> Result<DataPoint> {
        let timestamp = Timestamp::from_second(series.time.timestamp())?;

        let precipitation_amount = series
            .data
            .next_1_hours
            .as_ref()
            .and_then(|next| {
                next.details
                    .as_ref()
                    .and_then(|details| details.precipitation_amount)
            })
            .unwrap_or_default();

        let details = &series.data.instant.details;

        Ok(DataPoint {
            air_temperature: details.air_temperature.unwrap_or_default(),
            cloud_area_fraction: details.cloud_area_fraction.unwrap_or_default(),
            precipitation_amount,
            timestamp,
            wind_from_direction: details.wind_from_direction.unwrap_or_default(),
            wind_speed: details.wind_speed.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Coords {
    pub longitude: f64,
    pub latitude: f64,
}
