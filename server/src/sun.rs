use crate::error::Result;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

pub(crate) fn next_sunrise(
    latitude: f64,
    longitude: f64,
    timestamp: Timestamp,
) -> Result<Timestamp> {
    next_sunrise_sunset(latitude, longitude, timestamp, |(sunrise, _)| sunrise)
}

pub(crate) fn next_sunset(
    latitude: f64,
    longitude: f64,
    timestamp: Timestamp,
) -> Result<Timestamp> {
    next_sunrise_sunset(latitude, longitude, timestamp, |(_, sunset)| sunset)
}

fn next_sunrise_sunset<F>(
    latitude: f64,
    longitude: f64,
    timestamp: Timestamp,
    f: F,
) -> Result<Timestamp>
where
    F: Fn((Timestamp, Timestamp)) -> Timestamp,
{
    let next = sunrise_sunset(latitude, longitude, timestamp).map(&f)?;
    if next >= timestamp {
        return Ok(next);
    }

    let next_day = timestamp.checked_add(24.hours())?;
    sunrise_sunset(latitude, longitude, next_day).map(&f)
}

fn sunrise_sunset(
    latitude: f64,
    longitude: f64,
    timestamp: Timestamp,
) -> Result<(Timestamp, Timestamp)> {
    let date = timestamp.to_zoned(TimeZone::UTC);
    let (sunrise_secs, sunset_secs) = sunrise::sunrise_sunset(
        latitude,
        longitude,
        date.year().into(),
        date.month().try_into().expect("invalid month"),
        date.day().try_into().expect("invalid day"),
    );

    let sunrise = Timestamp::from_second(sunrise_secs)?;
    let sunset = Timestamp::from_second(sunset_secs)?;

    Ok((sunrise, sunset))
}
