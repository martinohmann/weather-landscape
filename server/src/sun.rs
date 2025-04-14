//! Helpers to calculate the time of sun phases.
use jiff::{Timestamp, ToSpan};
pub use sun::SunPhase;

/// Provides the timestamps of sun phases for a certain location.
#[derive(Debug, Clone, Copy)]
pub struct Sun {
    lat: f64,
    lon: f64,
    alt: Option<f64>,
}

impl Sun {
    /// Creates a new `Sun` for the location at `lat`/`lon` with optional altitude.
    pub fn new(lat: f64, lon: f64, alt: Option<f64>) -> Self {
        Sun { lat, lon, alt }
    }

    /// Calculates the time for the next [`SunPhase`] relative to the given date. The returned
    /// `Timestamp` is guaranteed to be greater that `ts`.
    pub fn next_phase(&self, ts: Timestamp, phase: SunPhase) -> Timestamp {
        let phase_ts = self.phase(ts, phase);
        if phase_ts > ts {
            return phase_ts;
        }

        let next_day = ts.checked_add(24.hours()).expect("timestamp overflow");
        self.phase(next_day, phase)
    }

    /// Calculates the time for the given [`SunPhase`] at a given date.
    pub fn phase(&self, ts: Timestamp, phase: SunPhase) -> Timestamp {
        let now_ms = ts.as_millisecond();
        let phase_ms =
            sun::time_at_phase(now_ms, phase, self.lat, self.lon, self.alt.unwrap_or(0.0));
        Timestamp::from_millisecond(phase_ms).expect("timestamp out of bounds")
    }

    /// Returns `true` if `ts` is between the [`SunPhase`]s given by `start` and `end`.
    ///
    /// The `end` [`SunPhase`] needs to happens after `start`, this method will always return
    /// `false`.
    pub fn is_between(&self, ts: Timestamp, start: SunPhase, end: SunPhase) -> bool {
        let start_ts = self.phase(ts, start);
        let end_ts = self.phase(ts, end);
        start_ts < ts && ts < end_ts
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn phases() {
        use SunPhase::*;

        let sun = Sun::new(52.0, 13.0, None);
        let date = ts("2024-10-25T15:14:00Z");

        // Phase did not happen yet on `date`.
        assert_eq!(sun.phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));
        assert_eq!(sun.next_phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));

        // Phase already happened on `date`.
        assert_eq!(sun.phase(date, Dawn), ts("2024-10-25T05:16:54.881Z"));
        assert_eq!(sun.next_phase(date, Dawn), ts("2024-10-26T05:18:36.694Z"));
    }

    #[test]
    fn is_between() {
        use SunPhase::*;

        let sun = Sun::new(52.0, 13.0, None);
        let date = ts("2024-10-25T15:14:00Z");

        assert!(sun.is_between(date, NightEnd, Night));
        assert!(!sun.is_between(date, Sunset, Dusk));
        assert!(!sun.is_between(date, Dusk, Night));
        assert!(sun.is_between(date, Sunrise, Sunset));
        // Sunrise always happens before sunset, so the date cannot be inbetween the two.
        assert!(!sun.is_between(date, Sunset, Sunrise));
    }
}
