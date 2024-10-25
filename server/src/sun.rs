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
    pub fn next_phase<T: Into<Timestamp>>(&self, ts: T, phase: SunPhase) -> Timestamp {
        let ts = ts.into();
        let phase_time = self.phase(ts, phase);
        if phase_time > ts {
            return phase_time;
        }

        let next_day = ts.checked_add(24.hours()).expect("timestamp overflow");
        self.phase(next_day, phase)
    }

    /// Calculates the time for the given [`SunPhase`] at a given date.
    pub fn phase<T: Into<Timestamp>>(&self, ts: T, phase: SunPhase) -> Timestamp {
        let ts = ts.into();
        let now_ms = ts.as_millisecond();
        let phase_ms = sun::time_at_phase(now_ms, phase, self.lat, self.lon, 0.0);
        Timestamp::from_millisecond(phase_ms).expect("timestamp out of bounds")
    }

    /// Returns `true` if `ts` is before the given [`SunPhase`] on the same day.
    pub fn is_before<T: Into<Timestamp>>(&self, ts: T, phase: SunPhase) -> bool {
        let ts = ts.into();
        let phase_ts = self.phase(ts, phase);
        ts < phase_ts
    }

    /// Returns `true` if `ts` is after the given [`SunPhase`] on the same day.
    pub fn is_after<T: Into<Timestamp>>(&self, ts: T, phase: SunPhase) -> bool {
        let ts = ts.into();
        let phase_ts = self.phase(ts, phase);
        phase_ts < ts
    }

    /// Returns `true` if `ts` is between the [`SunPhase`]s given by `start` and `end`.
    ///
    /// The `start` [`SunPhase`] needs to happen before `end`.
    ///
    /// # Panics
    ///
    /// If `start` >= `end`, this method panics.
    pub fn is_between<T: Into<Timestamp>>(&self, ts: T, start: SunPhase, end: SunPhase) -> bool {
        let ts = ts.into();
        let start_ts = self.phase(ts, start);
        let end_ts = self.phase(ts, end);

        if end_ts < start_ts {
            panic!("unexpected sun phase order: {end:?} happens before {start:?}, but the argument order does not match this relation");
        }

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

        let sun = Sun::new(52.0, 13.0);
        let date = ts("2024-10-25T15:14:00Z");

        // Phase did not happen yet on `date`.
        assert_eq!(sun.phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));
        assert_eq!(sun.next_phase(date, Sunset), ts("2024-10-25T15:54:39.775Z"));

        // Phase already happened on `date`.
        assert_eq!(sun.phase(date, Dawn), ts("2024-10-25T05:16:54.881Z"));
        assert_eq!(sun.next_phase(date, Dawn), ts("2024-10-26T05:18:36.694Z"));
    }

    #[test]
    fn is_before_or_after() {
        use SunPhase::*;

        let sun = Sun::new(52.0, 13.0);
        let date = ts("2024-10-25T16:14:00Z");

        assert!(!sun.is_before(date, NightEnd));
        assert!(!sun.is_before(date, Sunset));
        assert!(sun.is_before(date, Dusk));
        assert!(sun.is_before(date, Night));

        assert!(sun.is_after(date, NightEnd));
        assert!(sun.is_after(date, Sunset));
        assert!(!sun.is_after(date, Dusk));
        assert!(!sun.is_after(date, Night));
    }

    #[test]
    fn is_between() {
        use SunPhase::*;

        let sun = Sun::new(52.0, 13.0);
        let date = ts("2024-10-25T15:14:00Z");

        assert!(sun.is_between(date, NightEnd, Night));
        assert!(sun.is_between(date, Sunrise, Sunset));
        assert!(!sun.is_between(date, Sunset, Dusk));
        assert!(!sun.is_between(date, Dusk, Night));
    }

    #[test]
    #[should_panic]
    fn is_between_panic() {
        use SunPhase::*;

        let sun = Sun::new(52.0, 13.0);
        let date = ts("2024-10-25T15:14:00Z");

        // Sunrise always happens before sunset.
        assert!(sun.is_between(date, Sunset, Sunrise));
    }
}
