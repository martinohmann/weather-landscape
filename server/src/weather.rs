use crate::error::Result;
use jiff::{Timestamp, Zoned};
use monsoon::{body::Body, Monsoon, Params, Response};
use std::time::Duration;
use tower::limit::{ConcurrencyLimit, RateLimit};
use tower::{Service, ServiceBuilder, ServiceExt};

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
pub struct Weather {
    lat: f64,
    lon: f64,
    service: ConcurrencyLimit<RateLimit<Monsoon>>,
    last_response: Option<Response>,
}

impl Weather {
    /// Create a new weather service for the location at `lat`/`lon`.
    pub fn new(lat: f64, lon: f64) -> Result<Self> {
        let monsoon = Monsoon::new(USER_AGENT)?;

        // Limit request volume according to the met.no TOS: https://api.met.no/doc/TermsOfService.
        let service = ServiceBuilder::new()
            .concurrency_limit(10)
            .rate_limit(20, Duration::from_secs(1))
            .service(monsoon);

        Ok(Weather {
            lat,
            lon,
            service,
            last_response: None,
        })
    }

    /// Retrieve the weather forecast.
    pub async fn get_forecast(&mut self) -> Result<Response> {
        let response = self
            .service
            .ready()
            .await?
            .call(Params::new_with_last_response(
                self.lat,
                self.lon,
                None,
                self.last_response.clone(),
            )?)
            .await?;

        self.last_response = Some(response.clone());

        Ok(response)
    }

    /// Get sunrise and sunset for a date.
    pub fn sunrise_sunset(&self, date: &Zoned) -> Result<(Timestamp, Timestamp)> {
        let (sunrise_secs, sunset_secs) = sunrise::sunrise_sunset(
            self.lat,
            self.lon,
            date.year().into(),
            date.month().try_into().expect("invalid month"),
            date.day().try_into().expect("invalid day"),
        );

        let sunrise = Timestamp::from_second(sunrise_secs)?;
        let sunset = Timestamp::from_second(sunset_secs)?;

        Ok((sunrise, sunset))
    }

    fn build_forecast(&self) {}
}
