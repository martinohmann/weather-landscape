//! Helpers to calculate the time of sun phases.
use jiff::{Timestamp, ToSpan};
use sun::SunPhase;

/// Provides the timestamps of sun phases for a certain location.
#[derive(Debug, Clone, Copy)]
pub struct Sun {
    lat: f64,
    lon: f64,
}

impl Sun {
    /// Creates a new `Sun` for the location at `lat`/`lon`.
    pub fn new(lat: f64, lon: f64) -> Self {
        Sun { lat, lon }
    }

    /// Calculates the time for the next [`SunPhase`] relative to the given date. The returned
    /// `Timestamp` is guaranteed to be greater that `ts`.
    pub fn next_phase<T: Into<Timestamp>>(&self, ts: T, sun_phase: SunPhase) -> Timestamp {
        let ts = ts.into();
        let phase_time = self.phase(ts, sun_phase);
        if phase_time > ts {
            return phase_time;
        }

        let next_day = ts.checked_add(24.hours()).expect("timestamp overflow");
        self.phase(next_day, sun_phase)
    }

    /// Calculates the time for the given [`SunPhase`] at a given date.
    pub fn phase<T: Into<Timestamp>>(&self, ts: T, sun_phase: SunPhase) -> Timestamp {
        let ts = ts.into();
        let now_ms = ts.as_millisecond();
        let phase_ms = sun::time_at_phase(now_ms, sun_phase, self.lat, self.lon, 0.0);
        Timestamp::from_millisecond(phase_ms).expect("timestamp out of bounds")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sun() {
        use SunPhase::*;

        let ts = |s: &str| -> Timestamp { s.parse().unwrap() };
        let sun = Sun::new(52.0, 13.0);
        let date = ts("2024-10-25T15:14:00Z");

        // Phase did not happen yet on `date`.
        assert_eq!(sun.phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));
        assert_eq!(sun.next_phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));

        // Phase already happened on `date`.
        assert_eq!(sun.phase(date, Dawn), ts("2024-10-25T05:16:54.881Z"));
        assert_eq!(sun.next_phase(date, Dawn), ts("2024-10-26T05:18:36.694Z"));
    }
}
