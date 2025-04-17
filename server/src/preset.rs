use crate::config::PresetConfig;
use actix_web::HttpResponseBuilder;
use jiff::civil::{Date, DateTime, Time};
use std::collections::BTreeMap;
use tracing::info;

const HEADER_X_ESP_DEEP_SLEEP_SECONDS: &str = "x-esp-deep-sleep-seconds";

/// A time interval with start and end time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Interval {
    /// The optional date at which the interval starts.
    start_date: Option<Date>,
    /// The start time of the interval.
    start_time: Time,
    /// The optional date at which the interval ends.
    end_date: Option<Date>,
    /// The end time of the interval.
    end_time: Time,
}

impl Interval {
    /// Creates an `Interval` from a `PresetConfig`.
    fn from_config(config: &PresetConfig) -> Interval {
        Interval {
            start_date: config.start_date,
            start_time: config.start_time,
            end_date: config.end_date,
            end_time: config.end_time,
        }
    }

    /// Returns `true` if the instant falls within the interval, `false` otherwise.
    fn contains(&self, instant: DateTime) -> bool {
        let date = instant.date();
        let time = instant.time();

        if let Some(start_date) = self.start_date {
            if date < start_date {
                return false;
            }
        }

        if let Some(end_date) = self.end_date {
            if date > end_date {
                return false;
            }
        }

        if self.start_time < self.end_time {
            // Start and end time are assumed to be on the same day.
            time >= self.start_time && time < self.end_time
        } else {
            // Time window wraps over midnight.
            time >= self.start_time || time < self.end_time
        }
    }
}

/// A time-based preset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    fn from_config(name: &str, config: &PresetConfig) -> Preset {
        Preset {
            name: name.to_string(),
            interval: Interval::from_config(config),
            settings: Settings::from_config(config),
        }
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
    fn from_config(config: &PresetConfig) -> Settings {
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
    pub fn new(configs: &BTreeMap<String, PresetConfig>) -> Presets {
        let presets = configs
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, config)| Preset::from_config(name, config))
            .collect();

        Presets(presets)
    }

    /// Get the preset settings for a given datetime.
    ///
    /// If there are multiple presets for the time they are merged.
    ///
    /// Returns the `Default` settings if there are no presets for the given time.
    pub fn get_settings_for(&self, time: DateTime) -> Settings {
        self.0
            .iter()
            .filter(|preset| preset.interval.contains(time))
            .map(|preset| &preset.settings)
            .fold(Settings::default(), |acc, other| acc.merge(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::time;

    macro_rules! interval {
        ($start_time:expr, $end_time:expr) => {
            Interval {
                start_time: $start_time,
                end_time: $end_time,
                ..Default::default()
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
                ..Default::default()
            },
            Preset {
                interval: interval!(time(20, 0, 0, 0), time(23, 30, 0, 0)),
                ..Default::default()
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
                    esp_deep_sleep_seconds: None,
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
                    wreck_havoc: None,
                    esp_deep_sleep_seconds: Some(20),
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
                enabled: false,
                start_date: None,
                start_time: time(1, 0, 0, 0),
                end_date: None,
                end_time: time(1, 0, 0, 0),
                wreck_havoc: None,
                esp_deep_sleep_seconds: None,
            },
        );
        configs.insert(
            "preset2".into(),
            PresetConfig {
                start_date: None,
                enabled: true,
                start_time: time(1, 0, 0, 0),
                end_date: None,
                end_time: time(1, 0, 0, 0),
                wreck_havoc: None,
                esp_deep_sleep_seconds: Some(10),
            },
        );

        let presets = Presets::new(&configs);
        assert_eq!(
            presets.0,
            vec![Preset {
                name: "preset2".into(),
                interval: interval!(time(1, 0, 0, 0), time(1, 0, 0, 0)),
                settings: Settings {
                    esp_deep_sleep_seconds: Some(10),
                    wreck_havoc: None
                }
            }]
        );
    }
}
