use epd_waveshare::epd2in9_v2::HEIGHT;
use jiff::Timestamp;

const SECONDS_DAY: f64 = 24.0 * 60.0 * 60.0;

pub(super) fn timestamp_to_column(start: Timestamp, ts: Timestamp) -> i64 {
    debug_assert!(ts >= start, "timestamp needs be in the future");
    let delta = ts.duration_since(start).as_secs_f64();

    let column = ((delta / SECONDS_DAY) * (HEIGHT as f64)) as i64;
    column.min(HEIGHT as i64 - 1).max(0)
}
