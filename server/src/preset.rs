use crate::{
    config::PresetConfig,
    error::{Error, Result},
};
use actix_web::HttpResponseBuilder;
use jiff::civil::{Date, DateTime, Time};
use serde::Deserialize;
use std::collections::BTreeMap;
use tracing::info;

const HEADER_X_ESP_DEEP_SLEEP_SECONDS: &str = "x-esp-deep-sleep-seconds";

/// A moment in time.
#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Moment {
    /// A time on a specific date.
    DateTime(DateTime),
    /// A specific date.
    Date(Date),
    /// A time on any date.
    Time(Time),
}

impl Default for Moment {
    fn default() -> Self {
        Moment::DateTime(DateTime::default())
    }
}

/// A time interval with a start and end.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Interval {
    DateTime { from: DateTime, to: DateTime },
    Date { from: Date, to: Date },
    Time { from: Time, to: Time },
}

impl Interval {
    /// Creates an `Interval` from a `PresetConfig`.
    fn new(config: &PresetConfig) -> Result<Interval> {
        match (config.from, config.to) {
            (Moment::DateTime(from), Moment::DateTime(to)) => Ok(Interval::DateTime { from, to }),
            (Moment::Date(from), Moment::Date(to)) => Ok(Interval::Date { from, to }),
            (Moment::Time(from), Moment::Time(to)) => Ok(Interval::Time { from, to }),
            (_, _) => Err(Error::new(
                "`from` and `to` must be both either datetimes, dates or times",
            )),
        }
    }

    /// Returns `true` if the instant falls within the interval, `false` otherwise.
    fn contains(&self, instant: DateTime) -> bool {
        match *self {
            Interval::DateTime { from, to } => instant >= from && instant < to,
            Interval::Date { from, to } => {
                let date = instant.date();
                date >= from && date < to
            }
            Interval::Time { from, to } => {
                let time = instant.time();
                if from < to {
                    // Start and end time are assumed to be on the same day.
                    time >= from && time < to
                } else {
                    // Time window wraps over midnight.
                    time >= from || time < to
                }
            }
        }
    }
}

/// A time-based preset.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Preset {
    /// The name of the preset.
    name: String,
    /// The time interval in which the preset is active.
    interval: Interval,
    /// Preset settings.
    settings: Settings,
}

impl Preset {
    /// Creates a `Preset` from a name and a `PresetConfig`.
    fn new(name: &str, config: &PresetConfig) -> Result<Preset> {
        Ok(Preset {
            name: name.to_string(),
            interval: Interval::new(config)?,
            settings: Settings::new(config),
        })
    }
}

/// Preset settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    /// Deep sleep time configuration sent back to the ESP the next time it requests a new image.
    pub esp_deep_sleep_seconds: Option<u64>,
    /// Whether to add a lot of randomness into the weather data.
    pub wreck_havoc: Option<bool>,
}

impl Settings {
    /// Configures an HTTP response according to the settings.
    pub fn configure_response(&self, resp: &mut HttpResponseBuilder) {
        if let Some(seconds) = self.esp_deep_sleep_seconds {
            info!(?seconds, "sending custom ESP deep sleep configuration");
            resp.insert_header((HEADER_X_ESP_DEEP_SLEEP_SECONDS, seconds));
        }
    }

    /// Creates `Settings` from a `PresetConfig`.
    fn new(config: &PresetConfig) -> Settings {
        Settings {
            esp_deep_sleep_seconds: config.esp_deep_sleep_seconds,
            wreck_havoc: config.wreck_havoc,
        }
    }

    /// Merges `self` on top of `other` and returns the new `Settings`.
    fn merge(&self, other: &Settings) -> Settings {
        Settings {
            esp_deep_sleep_seconds: other.esp_deep_sleep_seconds.or(self.esp_deep_sleep_seconds),
            wreck_havoc: other.wreck_havoc.or(self.wreck_havoc),
        }
    }
}

/// Container for time-based presets.
#[derive(Debug, Clone)]
pub struct Presets(Vec<Preset>);

impl Presets {
    /// Creates a new `Presets` from a map of preset configs.
    pub fn new(configs: &BTreeMap<String, PresetConfig>) -> Result<Presets> {
        let presets = configs
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, config)| Preset::new(name, config))
            .collect::<Result<_>>()?;

        Ok(Presets(presets))
    }

    /// Get the preset settings for a given datetime.
    ///
    /// If there are multiple presets for the time they are merged.
    ///
    /// Returns the `Default` settings if there are no presets for the given time.
    pub fn get_settings_for(&self, instant: DateTime) -> Settings {
        self.0
            .iter()
            .filter(|preset| preset.interval.contains(instant))
            .fold(Settings::default(), |settings, preset| {
                settings.merge(&preset.settings)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::time;

    macro_rules! interval {
        ($start_time:expr, $end_time:expr) => {
            Interval::Time {
                from: $start_time,
                to: $end_time,
            }
        };
    }

    #[test]
    fn interval() {
        let interval = interval!(time(0, 30, 0, 0), time(23, 30, 0, 0));
        assert!(!interval.contains(time(23, 30, 0, 0).on(2025, 1, 1)));
        assert!(!interval.contains(time(0, 0, 0, 0).on(2025, 1, 1)));
        assert!(interval.contains(time(0, 30, 0, 0).on(2025, 1, 1)));
        assert!(interval.contains(time(1, 0, 0, 0).on(2025, 1, 1)));
    }

    #[test]
    fn interval_wraps_over_midnight() {
        let interval = interval!(time(23, 30, 0, 0), time(0, 30, 0, 0));
        assert!(interval.contains(time(23, 30, 0, 0).on(2025, 1, 1)));
        assert!(interval.contains(time(0, 0, 0, 0).on(2025, 1, 1)));
        assert!(!interval.contains(time(0, 30, 0, 0).on(2025, 1, 1)));
        assert!(!interval.contains(time(1, 0, 0, 0).on(2025, 1, 1)));
    }

    #[test]
    fn interval_24hours() {
        let interval = interval!(time(0, 0, 0, 0), time(0, 0, 0, 0));
        assert!(interval.contains(time(0, 0, 0, 0).on(2025, 1, 1)));
        assert!(interval.contains(time(1, 0, 0, 0).on(2025, 1, 1)));
        assert!(interval.contains(time(23, 0, 0, 0).on(2025, 1, 1)));
    }

    #[test]
    fn empty_presets() {
        assert_eq!(
            Presets(Vec::new()).get_settings_for(time(0, 0, 0, 0).on(2025, 1, 1)),
            Settings::default()
        );
    }

    #[test]
    fn presets() {
        let settings = Settings {
            wreck_havoc: Some(true),
            ..Default::default()
        };

        let presets = Presets(vec![
            Preset {
                interval: interval!(time(23, 30, 0, 0), time(0, 30, 0, 0)),
                settings: settings.clone(),
                name: "preset1".into(),
            },
            Preset {
                interval: interval!(time(20, 0, 0, 0), time(23, 30, 0, 0)),
                settings: Settings::default(),
                name: "preset2".into(),
            },
        ]);

        assert_eq!(
            presets.get_settings_for(time(23, 30, 0, 0).on(2025, 1, 1)),
            settings.clone()
        );
        assert_eq!(
            presets.get_settings_for(time(23, 29, 59, 0).on(2025, 1, 1)),
            Settings::default()
        );
        assert_eq!(
            presets.get_settings_for(time(19, 59, 59, 999).on(2025, 1, 1)),
            Settings::default()
        );
    }

    #[test]
    fn merge_overlapping_presets() {
        let presets = Presets(vec![
            Preset {
                name: "preset1".into(),
                interval: interval!(time(23, 30, 0, 0), time(0, 30, 0, 0)),
                settings: Settings {
                    wreck_havoc: Some(true),
                    ..Default::default()
                },
            },
            Preset {
                name: "preset2".into(),
                interval: interval!(time(0, 0, 0, 0), time(2, 0, 0, 0)),
                settings: Settings {
                    wreck_havoc: Some(false),
                    esp_deep_sleep_seconds: Some(10),
                },
            },
            Preset {
                name: "preset3".into(),
                interval: interval!(time(0, 10, 0, 0), time(1, 0, 0, 0)),
                settings: Settings {
                    esp_deep_sleep_seconds: Some(20),
                    ..Default::default()
                },
            },
        ]);

        assert_eq!(
            presets.get_settings_for(time(0, 15, 0, 0).on(2025, 1, 1)),
            Settings {
                wreck_havoc: Some(false),
                esp_deep_sleep_seconds: Some(20),
            }
        );
    }

    #[test]
    fn presets_new() {
        let mut configs: BTreeMap<String, PresetConfig> = BTreeMap::new();
        configs.insert(
            "my-preset1".into(),
            PresetConfig {
                from: Moment::Time(time(1, 0, 0, 0)),
                to: Moment::Time(time(1, 0, 0, 0)),
                ..Default::default()
            },
        );
        configs.insert(
            "preset2".into(),
            PresetConfig {
                enabled: true,
                from: Moment::Time(time(1, 0, 0, 0)),
                to: Moment::Time(time(1, 0, 0, 0)),
                esp_deep_sleep_seconds: Some(10),
                ..Default::default()
            },
        );

        let presets = Presets::new(&configs).unwrap();
        assert_eq!(
            presets.0,
            vec![Preset {
                name: "preset2".into(),
                interval: interval!(time(1, 0, 0, 0), time(1, 0, 0, 0)),
                settings: Settings {
                    esp_deep_sleep_seconds: Some(10),
                    ..Default::default()
                }
            }]
        );
    }
}
