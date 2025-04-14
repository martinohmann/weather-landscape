use crate::error::{Error, Result};
use jiff::Timestamp;
use monsoon::{
    Monsoon, Params, Response,
    body::{Body, TimeSeries},
};
use rand::{Rng, seq::SliceRandom};
use std::time::Duration;
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tower::{
    Service, ServiceBuilder, ServiceExt,
    limit::{ConcurrencyLimit, RateLimit},
};
use tracing::info;

// Met.no requires to identify oneself via user-agent header. This is best practice anyways.
const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

#[derive(Debug)]
struct WeatherInner {
    service: ConcurrencyLimit<RateLimit<Monsoon>>,
    last_response: Option<Response>,
    latitude: f64,
    longitude: f64,
    altitude: Option<i32>,
}

impl WeatherInner {
    fn new(latitude: f64, longitude: f64, altitude: Option<i32>) -> Result<Self> {
        let monsoon = Monsoon::new(USER_AGENT)?;

        // Limit request volume according to the met.no TOS: https://api.met.no/doc/TermsOfService.
        let service = ServiceBuilder::new()
            .concurrency_limit(10)
            .rate_limit(20, Duration::from_secs(1))
            .service(monsoon);

        Ok(WeatherInner {
            service,
            last_response: None,
            latitude,
            longitude,
            altitude,
        })
    }

    async fn get(&mut self) -> Result<WeatherData> {
        let response = self
            .service
            .ready()
            .await?
            .call(Params::new_with_last_response(
                self.latitude,
                self.longitude,
                self.altitude,
                self.last_response.clone(),
            )?)
            .await?;

        let body = response.body()?;
        let data = WeatherData::from_body(&body)?;

        self.last_response = Some(response);

        Ok(data)
    }
}

#[derive(Debug, Clone)]
pub struct Weather {
    inner: Arc<Mutex<WeatherInner>>,
}

impl Weather {
    /// Create a new weather service for the location at `latitude`/`longitude` with optional
    /// altitude.
    pub fn new(latitude: f64, longitude: f64, altitude: Option<i32>) -> Result<Self> {
        let inner = WeatherInner::new(latitude, longitude, altitude)?;
        let inner = Arc::new(Mutex::new(inner));
        Ok(Weather { inner })
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
                altitude: body.geometry.coordinates.altitude,
            },
            current,
            forecasts,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct DataPoint {
    pub air_pressure_at_sea_level: f64,
    pub air_temperature: f64,
    pub cloud_area_fraction: f64,
    pub condition: Condition,
    pub fog_area_fraction: f64,
    pub precipitation_amount: f64,
    pub probability_of_thunder: f64,
    pub timestamp: Timestamp,
    pub wind_from_direction: f64,
    pub wind_speed: f64,
}

impl DataPoint {
    fn from_time_series(series: &TimeSeries) -> Result<DataPoint> {
        let timestamp = Timestamp::from_second(series.time.timestamp())?;

        let (condition, precipitation_amount, probability_of_thunder) = series
            .data
            .next_1_hours
            .as_ref()
            .map(|next| {
                let condition = Condition::from_str(next.summary.symbol_code).ok();

                let (precipitation_amount, probability_of_thunder) = next
                    .details
                    .as_ref()
                    .map(|details| (details.precipitation_amount, details.probability_of_thunder))
                    .unwrap_or_default();

                (condition, precipitation_amount, probability_of_thunder)
            })
            .unwrap_or_default();

        let details = &series.data.instant.details;

        Ok(DataPoint {
            air_pressure_at_sea_level: details.air_pressure_at_sea_level.unwrap_or_default(),
            air_temperature: details.air_temperature.unwrap_or_default(),
            cloud_area_fraction: details.cloud_area_fraction.unwrap_or_default(),
            condition: condition.unwrap_or_default(),
            fog_area_fraction: details.fog_area_fraction.unwrap_or_default(),
            precipitation_amount: precipitation_amount.unwrap_or_default(),
            probability_of_thunder: probability_of_thunder
                .map(|p| (p / 100.0).clamp(0.0, 1.0))
                .unwrap_or_default(),
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
    pub altitude: f64,
}

#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub enum Condition {
    ClearSky,
    Cloudy,
    Fair,
    Fog,
    PartlyCloudy,
    Rain,
    RainAndThunder,
    Sleet,
    SleetAndThunder,
    Snow,
    SnowAndThunder,
    #[default]
    Unknown,
}

impl Condition {
    pub fn has_sleet(&self) -> bool {
        matches!(self, Condition::Sleet | Condition::SleetAndThunder)
    }

    pub fn has_snow(&self) -> bool {
        matches!(self, Condition::Snow | Condition::SnowAndThunder)
    }

    pub fn has_thunder(&self) -> bool {
        matches!(
            self,
            Condition::RainAndThunder | Condition::SleetAndThunder | Condition::SnowAndThunder
        )
    }
}

impl FromStr for Condition {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // We don't distiguish between day and night conditions, and we also don't care about
        // "light" and "heavy" for now.
        let normalized = s
            .trim_end_matches("_day")
            .trim_end_matches("_night")
            .trim_start_matches("light")
            .trim_start_matches("heavy");

        // Conditions from https://github.com/metno/weathericons/tree/main/weather
        let condition = match normalized {
            "clearsky" => Condition::ClearSky,
            "cloudy" => Condition::Cloudy,
            "fair" => Condition::Fair,
            "fog" => Condition::Fog,
            "partlycloudy" => Condition::PartlyCloudy,
            "rain" | "rainshowers" => Condition::Rain,
            "rainandthunder" | "rainshowersandthunder" => Condition::RainAndThunder,
            "sleet" | "sleetshowers" => Condition::Sleet,
            "sleetandthunder" | "sleetshowersandthunder" => Condition::SleetAndThunder,
            "snow" | "snowshowers" => Condition::Snow,
            "snowandthunder" | "snowshowersandthunder" => Condition::SnowAndThunder,
            // Typing errors.
            "ssleetshowersandthunder" => Condition::SleetAndThunder,
            "ssnowshowersandthunder" => Condition::SnowAndThunder,
            _ => return Err(Error::new(format!("unknown weather condition: {}", s))),
        };

        Ok(condition)
    }
}

/// Adds a lot of randomness to the weather data. This is useful for testing.
pub fn cause_havoc(weather: &mut WeatherData) {
    info!("causing havoc in the weather data");

    let conditions = &[
        Condition::Fog,
        Condition::Rain,
        Condition::RainAndThunder,
        Condition::Sleet,
        Condition::SleetAndThunder,
        Condition::Snow,
        Condition::SnowAndThunder,
    ];

    let mut rng = rand::thread_rng();

    let mut add_randomness = |data: &mut DataPoint| {
        data.air_pressure_at_sea_level += rng.gen_range(-200.0f64..=200.0).clamp(0.0, 2000.0);
        data.air_temperature += rng.gen_range(-2.0..=2.0);
        data.cloud_area_fraction += rng.gen_range(-50.0f64..50.0).clamp(0.0, 100.0);
        data.condition = *conditions.choose(&mut rng).unwrap();
        data.fog_area_fraction += rng.gen_range(-50.0f64..50.0).clamp(0.0, 100.0);
        data.precipitation_amount += rng.gen_range(-5.0f64..5.0).clamp(0.0, 50.0);
        data.probability_of_thunder = rng.gen_range(0.0..1.0);
        data.wind_from_direction += rng.gen_range(-90.0f64..90.0).clamp(0.0, 360.0);
        data.wind_speed += rng.gen_range(-10.0f64..=10.0).max(0.0);
    };

    add_randomness(&mut weather.current);

    for data in &mut weather.forecasts {
        add_randomness(data);
    }
}
