use crate::error::Result;
use crate::sun;
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
    async fn forecast(&mut self) -> Result<Forecast> {
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

        Forecast::from_body(&body)
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

    /// Retrieve the weather forecast.
    pub async fn forecast(&self) -> Result<Forecast> {
        self.inner.lock().await.forecast().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct Forecast {
    pub hourly_forecast: Vec<HourlyForecast>,
    pub next_sunrise: Timestamp,
    pub next_sunset: Timestamp,
    pub timestamp: Timestamp,
}

impl Forecast {
    fn from_body(body: &Body) -> Result<Forecast> {
        let timestamp = Timestamp::now();
        let latitude = body.geometry.coordinates.latitude;
        let longitude = body.geometry.coordinates.longitude;

        let next_sunrise = sun::next_sunrise(latitude, longitude, timestamp)?;
        let next_sunset = sun::next_sunset(latitude, longitude, timestamp)?;

        let hourly_forecast = body
            .properties
            .timeseries
            .iter()
            // Only take the forecast of every 4th hour.
            .step_by(4)
            // Take 7 forecasts * 4h = 28h to be able to draw the temperature graph until the edge
            // of the screen towards the forecast from now+28h.
            .take(7)
            .map(HourlyForecast::from_time_series)
            .collect::<Result<Vec<_>>>()?;

        Ok(Forecast {
            timestamp,
            next_sunset,
            next_sunrise,
            hourly_forecast,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct HourlyForecast {
    pub air_temperature: f64,
    pub cloud_area_fraction: f64,
    pub precipitation_amount: f64,
    pub timestamp: Timestamp,
    pub wind_from_direction: f64,
    pub wind_speed: f64,
}

impl HourlyForecast {
    fn from_time_series(series: &TimeSeries) -> Result<HourlyForecast> {
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

        Ok(HourlyForecast {
            air_temperature: details.air_temperature.unwrap_or_default(),
            cloud_area_fraction: details.cloud_area_fraction.unwrap_or_default(),
            precipitation_amount,
            timestamp,
            wind_from_direction: details.wind_from_direction.unwrap_or_default(),
            wind_speed: details.wind_speed.unwrap_or_default(),
        })
    }
}
