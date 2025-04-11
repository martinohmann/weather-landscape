mod img;
mod sprites;

pub use self::img::{Image, ImageFormat};
use self::{
    img::{BLACK, TRANSPARENT, WHITE},
    sprites::{Sprite, sprite, spriten},
};
use crate::{
    app::Metrics,
    config::Config,
    sun::{Sun, SunPhase::*},
    weather::{Condition, DataPoint, WeatherData},
};
use epd_waveshare::epd2in9_v2::{HEIGHT, WIDTH};
use imageproc::drawing::BresenhamLineIter;
use jiff::{SignedDuration, Timestamp, civil::time, tz::TimeZone};
use rand::{Rng, seq::SliceRandom};
use std::collections::BTreeMap;
use tracing::debug;

/// Renders landscape images from weather data.
#[derive(Clone)]
pub struct Renderer {
    night_mode: bool,
    metrics: Metrics,
}

impl Renderer {
    /// Creates a new `Renderer` from config and metrics.
    pub fn new(config: &Config, metrics: Metrics) -> Self {
        Renderer {
            night_mode: !config.disable_night_mode,
            metrics,
        }
    }

    /// Renders the weather data into a landscape image.
    pub fn render(&self, data: &WeatherData) -> Image {
        let mut ctx = RenderContext::new(data);

        debug!(?data, "rendering image for weather data");

        self.draw_celestial_bodies(&mut ctx);
        self.draw_current_weather(&mut ctx, &data.current);
        self.draw_forecasts(&mut ctx, &data.forecasts);
        self.draw_midday_and_midnight(&mut ctx);

        // Draw the temperature graph.
        for (x, y) in ctx.temperature_graph {
            ctx.img.draw_pixel(x, y);
        }

        let dark_outside =
            ctx.sun.is_before(ctx.instant, Dawn) || ctx.sun.is_after(ctx.instant, Dusk);

        if self.night_mode && dark_outside {
            ctx.img.invert_pixels();
        }

        ctx.img
    }

    fn draw_house(&self, ctx: &mut RenderContext, weather: &DataPoint) {
        let twilight = ctx.sun.is_between(ctx.instant, Sunset, Night)
            || ctx.sun.is_between(ctx.instant, NightEnd, Sunrise);

        let house = if twilight {
            // It's dark outside, lights on.
            sprite("house_01")
        } else {
            // It's either day time or late at night, lights out in any case.
            sprite("house_00")
        };

        let y = ctx.temperature_to_y(weather.air_temperature) - house.height() as i64;

        self.draw_sprite(ctx, house, 0, y);
    }

    fn draw_celestial_bodies(&self, ctx: &mut RenderContext) {
        let sun = sprite("sun_00");
        let next_sunrise = ctx.sun.next_phase(ctx.instant, Sunrise);
        let sun_x = ctx.timestamp_to_x(next_sunrise) - (sun.width() / 2) as i64;

        self.draw_sprite(ctx, sun, sun_x, 0);

        let moon = sprite("moon_00");
        let next_sunset = ctx.sun.next_phase(ctx.instant, Sunset);
        let moon_x = ctx.timestamp_to_x(next_sunset) - (moon.width() / 4) as i64;

        self.draw_sprite(ctx, moon, moon_x, 0);
    }

    fn draw_midday_and_midnight(&self, ctx: &mut RenderContext) {
        self.draw_flower(ctx, "flower_00", 0);
        self.draw_flower(ctx, "flower_01", 12);
    }

    fn draw_flower(&self, ctx: &mut RenderContext, name: &str, hour: i8) {
        let local_time = ctx.instant.to_zoned(TimeZone::system());
        let mut time = local_time.with().time(time(hour, 0, 0, 0)).build().unwrap();
        if time < local_time {
            time = time.checked_add(SignedDuration::from_hours(24)).unwrap();
        }

        let x = ctx.timestamp_to_x(time.timestamp());

        if x < ctx.x_offset {
            // We don't want it to overlap with the house, or do we?
            return;
        }

        if let Some(&y) = ctx.temperature_graph.get(&x) {
            let sprite = sprite(name);
            let y = y - sprite.height() as i64;
            self.draw_sprite(ctx, sprite, x, y);
        }
    }

    fn draw_sky(&self, ctx: &mut RenderContext, data: &DataPoint, x: i64, width: i64) {
        self.draw_clouds(ctx, data, x, 5, width);
        self.draw_precipitation(ctx, data, x, ctx.cloud_height + 5, width);
        self.draw_fog(ctx, data, x, ctx.cloud_height + 10, width);
    }

    fn draw_current_weather(&self, ctx: &mut RenderContext, weather: &DataPoint) {
        self.draw_house(ctx, weather);
        self.draw_sky(ctx, weather, 0, ctx.x_offset);
        self.draw_temperature(ctx, weather.air_temperature, ctx.x_offset / 2);
    }

    fn draw_forecasts(&self, ctx: &mut RenderContext, forecasts: &[DataPoint]) {
        // Only draw a forecast sample for every 4 hours. It'll get too crowded otherwise.
        for (i, forecast) in forecasts.iter().enumerate().step_by(4) {
            let x = ctx.forecast_x(i);
            self.draw_sky(ctx, forecast, x, ctx.x_step * 4);
            self.draw_trees(ctx, forecast, x);
        }

        self.draw_temperature_extrema(ctx, forecasts, ctx.min_temperature);
        self.draw_temperature_extrema(ctx, forecasts, ctx.max_temperature);
    }

    fn draw_temperature_extrema(
        &self,
        ctx: &mut RenderContext,
        forecasts: &[DataPoint],
        temperature: f64,
    ) {
        if let Some((i, data_point)) = forecasts
            .iter()
            .enumerate()
            .find(|(_, dp)| dp.air_temperature == temperature)
        {
            let x = ctx.forecast_x(i);
            self.draw_temperature(ctx, data_point.air_temperature, x);
        }
    }

    fn draw_temperature(&self, ctx: &mut RenderContext, temperature: f64, x: i64) {
        let y = ctx.temperature_to_y(temperature);
        self.draw_number(ctx, x, y + 5, temperature.round() as i64);
    }

    fn draw_clouds(&self, ctx: &mut RenderContext, data: &DataPoint, x: i64, y: i64, width: i64) {
        let cloudset: &[usize] = match data.cloud_area_fraction {
            2.0..5.0 => &[2],
            5.0..10.0 => &[3, 2],
            10.0..20.0 => &[5, 3, 2],
            20.0..30.0 => &[10, 5],
            30.0..40.0 => &[10, 10],
            40.0..50.0 => &[10, 10, 5],
            50.0..60.0 => &[30, 5],
            60.0..70.0 => &[30, 10],
            70.0..80.0 => &[30, 10, 5, 5],
            80.0..90.0 => &[30, 10, 10],
            90.0.. => &[50, 30, 10, 10, 5],
            _ => &[],
        };

        let mut rng = rand::thread_rng();

        for &n in cloudset {
            let offset = rng.gen_range(0..width);
            let cloud = spriten("cloud", n);
            self.draw_sprite(ctx, cloud, x + offset, y);
        }
    }

    fn draw_fog(&self, ctx: &mut RenderContext, data: &DataPoint, x: i64, y: i64, width: i64) {
        let x_max = x + width;
        let Some(&y_max) = (x..x_max)
            .filter_map(|x| ctx.temperature_graph.get(&x))
            .min()
        else {
            return;
        };

        let fog_width = width / 2;
        let y_step = 6;
        let y_range = (y_max - y) / 2;
        let mut rng = rand::thread_rng();

        for y_off in (0..y_range).step_by(y_step) {
            let percentage = (y_off as f64 / y_range as f64) * 100.0;

            if data.fog_area_fraction <= percentage {
                break;
            }

            let x_start = x + rng.gen_range(3..fog_width / 2);
            let y_start = y + y_off;

            for i in 0..=fog_width {
                let x = x_start + i;
                let y = y_start + (i as f64 + 2.0).sin().round() as i64;

                ctx.img.draw_pixel(x, y);
            }

            self.metrics.object_counter("fog").inc();
        }
    }

    fn draw_precipitation(
        &self,
        ctx: &mut RenderContext,
        data: &DataPoint,
        x: i64,
        y: i64,
        width: i64,
    ) {
        if data.precipitation_amount <= 0.0 {
            // There's nothing that could fall from the sky.
            return;
        }

        let (heaviness, factor) = match data.condition {
            Condition::Snow => (5.0, 10.0),
            Condition::Sleet => (5.0, 15.0),
            _ => (5.0, 20.0),
        };

        let r = 1.0 - (data.precipitation_amount / heaviness) / factor;

        for x in x..x + width {
            if let Some(&y_max) = ctx.temperature_graph.get(&x) {
                for y in (y..y_max).step_by(2) {
                    if rand::random::<f64>() > r {
                        let snow = match data.condition {
                            Condition::Snow => true,
                            Condition::Sleet => rand::random(),
                            _ => false,
                        };

                        if snow {
                            ctx.img.draw_pixel(x, y);
                            self.metrics.object_counter("snowflake").inc();
                        } else {
                            ctx.img.draw_pixel(x, y);
                            ctx.img.draw_pixel(x, y - 1);
                            self.metrics.object_counter("raindrop").inc();
                        }
                    }
                }
            }
        }
    }

    fn draw_trees(&self, ctx: &mut RenderContext, data: &DataPoint, x: i64) {
        // @FIXME(mohmann): Simplify this complicated method.

        fn direction_distance(a: f64, b: f64) -> f64 {
            let high = a.max(b);
            let low = a.min(b);
            let mut distance = high - low;

            if distance > 180. {
                distance = 360. - distance
            }

            distance
        }

        fn select_trees<'a>(a: f64, b: f64, name: &'a str, trees: &mut Vec<&'a str>) {
            const TREE_COUNTS: &[usize] = &[4, 3, 3, 2, 2, 1, 1];

            let step = 11.25; // degrees
            let distance = direction_distance(a, b);
            let index = (distance / step) as usize;

            if index < TREE_COUNTS.len() {
                for _ in 0..TREE_COUNTS[index] {
                    trees.push(name);
                }
            }
        }

        const TREE_DIRECTIONS: [(&str, f64); 4] =
            [("pine", 0.), ("east", 90.), ("palm", 180.), ("tree", 270.)];

        let mut trees: Vec<&str> = Vec::new();

        for (name, direction) in TREE_DIRECTIONS {
            select_trees(data.wind_from_direction, direction, name, &mut trees);
        }

        let mut rng = rand::thread_rng();
        trees.shuffle(&mut rng);

        let wind_speed = data.wind_speed;

        let wind_indices: &[usize] = if wind_speed <= 0.4 {
            &[]
        } else if wind_speed <= 0.7 {
            &[0]
        } else if wind_speed <= 1.7 {
            &[1, 0, 0]
        } else if wind_speed <= 3.3 {
            &[1, 1, 0, 0]
        } else if wind_speed <= 5.2 {
            &[1, 2, 0, 0]
        } else if wind_speed <= 7.4 {
            &[1, 2, 2, 0]
        } else if wind_speed <= 9.8 {
            &[1, 2, 3, 0]
        } else if wind_speed <= 12.4 {
            &[2, 2, 3, 0]
        } else {
            &[3, 3, 3, 3]
        };

        let mut wind_indices = Vec::from_iter(wind_indices);
        wind_indices.shuffle(&mut rng);

        let mut x_offset = x;

        for (tree_index, &wind_index) in wind_indices.into_iter().enumerate() {
            let offset = x_offset + 5;

            if offset > ctx.temperature_graph.len() as i64 {
                break;
            }

            if let Some(name) = trees.get(tree_index) {
                let Some(y) = ctx.temperature_graph.get(&offset) else {
                    continue;
                };
                let tree = spriten(name, wind_index);
                let y_offset = (y - tree.height() as i64) + 1;
                self.draw_sprite(ctx, tree, x_offset, y_offset);
            }

            x_offset += 9;
        }
    }

    fn draw_number(&self, ctx: &mut RenderContext, x: i64, y: i64, value: i64) {
        let sign = if value >= 0 {
            sprite("digit_10") // plus
        } else {
            sprite("digit_11") // minus
        };

        // We're assuming that air temperatures values have at most 2 digits, anything else would
        // be highly concerning.
        let value = value.abs();
        let d1 = value / 10;
        let d2 = value % 10;

        let digits = if value < 10 { 1 } else { 2 };
        let digit_width = sign.width() as i64;

        // Center the digits, excluding the sign because it looks better.
        let mut offset = -(digits * (digit_width + 1) / 2) - digit_width;

        self.draw_sprite(ctx, sign, x + offset, y);
        offset += digit_width + 1;

        if d1 > 0 {
            let digit = spriten("digit", d1 as _);
            self.draw_sprite(ctx, digit, x + offset, y);
            offset += digit_width + 1;
        }

        let digit = spriten("digit", d2 as _);
        self.draw_sprite(ctx, digit, x + offset, y);
    }

    fn draw_sprite(&self, ctx: &mut RenderContext, sprite: &Sprite, x: i64, y: i64) {
        sprite.overlay(&mut ctx.img, x, y);
        self.metrics.object_counter(sprite.name()).inc();
    }
}

#[derive(Debug)]
struct RenderContext {
    img: Image,
    sun: Sun,
    // X-offset for the weather graph.
    x_offset: i64,
    // X-step for a single forecast.
    x_step: i64,
    // Y-offset for the weather graph.
    y_offset: i64,
    // Height of the clouds.
    cloud_height: i64,
    // The minimum temperature from the forecast.
    min_temperature: f64,
    // The maximum temperature from the forecast.
    max_temperature: f64,
    // Controls how many pixels to render per degree celsius.
    degrees_per_pixel: f64,
    // The instant at which the render context was created.
    instant: Timestamp,
    // The points for drawing the temperature graph.
    temperature_graph: BTreeMap<i64, i64>,
}

impl RenderContext {
    fn new(data: &WeatherData) -> Self {
        // We'll flip width and height here. The e-paper display works in portrait mode but we'd like
        // to draw the image in landscape mode, because it's more intiutive. The rendered image gets
        // rotated by 90 degrees before serving it to the esp32.
        let img = Image::new(HEIGHT, WIDTH);
        let (width, height) = img.dimensions();
        let x_offset = sprite("house_00").width() as i64;
        let x_step = (width as i64 - x_offset) / (data.forecasts.len() as i64 - 1);
        let y_step = (height as f64 * 0.39).round() as i64;
        let y_offset = (height as i64 / 2) + y_step;
        let cloud_height = sprite("cloud_02").height() as i64;
        let instant = Timestamp::now();

        let coords = &data.coords;
        let sun = Sun::new(coords.latitude, coords.longitude, Some(coords.altitude));

        let temperatures: Vec<f64> = data
            .forecasts
            .iter()
            // We'll ignore the last forecast in the temperature calculation because it's going to
            // be off-screen and is only used to draw the temperature graph to the edge of the
            // screen.
            .take(data.forecasts.len() - 1)
            .map(|fc| fc.air_temperature)
            .collect();

        let current_temperature = data.current.air_temperature;

        let min_temperature = temperatures
            .iter()
            .fold(current_temperature, |a, &b| a.min(b));
        let max_temperature = temperatures
            .iter()
            .fold(current_temperature, |a, &b| a.max(b));
        let temperature_range = max_temperature - min_temperature;

        let degrees_per_pixel = if temperature_range < y_step as f64 {
            0.5
        } else {
            temperature_range / y_step as f64
        };

        let mut ctx = RenderContext {
            img,
            sun,
            x_step,
            x_offset,
            y_offset,
            cloud_height,
            min_temperature,
            max_temperature,
            degrees_per_pixel,
            instant,
            temperature_graph: BTreeMap::new(),
        };

        ctx.populate_temperature_graph(data);
        ctx
    }

    fn timestamp_to_x(&self, timestamp: Timestamp) -> i64 {
        const SECONDS_DAY: f64 = 24.0 * 60.0 * 60.0;
        let delta = timestamp.duration_since(self.instant).as_secs_f64();
        let width = self.img.width() as f64 - self.x_offset as f64;
        ((delta / SECONDS_DAY) * width).round() as i64 + self.x_offset
    }

    fn temperature_to_y(&self, temperature: f64) -> i64 {
        let delta = temperature - self.min_temperature;
        self.y_offset - (delta / self.degrees_per_pixel).round() as i64
    }

    fn forecast_x(&self, i: usize) -> i64 {
        self.x_offset + (self.x_step * (i as i64 + 1))
    }

    fn forecast_coords(&self, i: usize, data_point: &DataPoint) -> (i64, i64) {
        let x = self.forecast_x(i);
        let y = self.temperature_to_y(data_point.air_temperature);
        (x, y)
    }

    fn populate_temperature_graph(&mut self, data: &WeatherData) {
        let collect_points =
            |graph: &mut BTreeMap<i64, i64>, x1: i64, y1: i64, x2: i64, y2: i64| {
                let (start, end) = ((x1 as f32, y1 as f32), (x2 as f32, y2 as f32));

                for (x, y) in BresenhamLineIter::new(start, end) {
                    graph.insert(x as i64, y as i64);
                }
            };

        // Collect points for the current temperature below the house.
        let y = self.temperature_to_y(data.current.air_temperature);

        collect_points(&mut self.temperature_graph, 0, y, self.x_offset - 1, y);

        // Collect points between the current temperature and the first forecasts.
        let (x1, y1) = (self.x_offset - 1, y);
        let (x2, y2) = self.forecast_coords(0, &data.forecasts[0]);

        collect_points(&mut self.temperature_graph, x1, y1, x2, y2);

        // Collect points between forecasts.
        for (i, window) in data.forecasts.windows(2).enumerate() {
            let (x1, y1) = self.forecast_coords(i, &window[0]);
            let (x2, y2) = self.forecast_coords(i + 1, &window[1]);

            collect_points(&mut self.temperature_graph, x1, y1, x2, y2);
        }
    }
}
