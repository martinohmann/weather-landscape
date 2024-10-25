mod curve;
mod sprites;

use self::curve::fit_curve_to_points;
use self::sprites::{sprite, spriten};
use crate::{
    error::{Error, Result},
    sun::Sun,
    weather::{Condition, DataPoint, WeatherData},
};
use anyhow::anyhow;
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    color::Color,
    epd2in9_v2::{HEIGHT, WIDTH},
    graphics::VarDisplay,
};
use flo_curves::Coord2;
use image::{imageops, ImageFormat, Pixel, Rgba, RgbaImage};
use indexmap::IndexMap;
use jiff::{civil::time, tz::TimeZone, SignedDuration, Timestamp};
use log::{debug, trace};
use rand::{seq::SliceRandom, Rng};
use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
};
use sun::SunPhase::*;

const HEAVY_RAIN: f64 = 5.0;
const RAIN_FACTOR: f64 = 20.0;
const HEAVY_SLEET: f64 = 5.0;
const SLEET_FACTOR: f64 = 15.0;
const HEAVY_SNOW: f64 = 5.0;
const SNOW_FACTOR: f64 = 10.0;
const SECONDS_DAY: f64 = 24.0 * 60.0 * 60.0;
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

pub fn render(data: &WeatherData) -> Result<Canvas> {
    // We'll flip width and height here. The e-paper display works in portrait mode but we'd like
    // to draw the image in landscape mode, because it's more intiutive. The rendered image gets
    // rotated by 90 degrees before serving it to the esp32.
    let mut canvas = Canvas::new(HEIGHT, WIDTH);
    let ctx = RenderContext::create(data, canvas.width(), canvas.height())?;

    debug!("rendering with context: {ctx:?}");

    let line_points = ctx.compute_line_points();

    canvas.draw_celestial_bodies(&ctx);
    canvas.draw_current_weather(&ctx, &line_points);
    canvas.draw_forecasts(&ctx, &line_points);
    canvas.draw_midday_and_midnight(&ctx, &line_points);

    // Draw the temperature graph.
    for (x, y) in line_points {
        canvas.draw_pixel(x, y);
    }

    if ctx.sun.is_before(ctx.instant, Dawn) || ctx.sun.is_after(ctx.instant, Dusk) {
        // Enable night mode.
        for pixel in canvas.pixels_mut() {
            pixel.invert();
        }
    }

    Ok(canvas)
}

#[derive(Debug)]
pub struct Canvas {
    img: RgbaImage,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Canvas {
            img: RgbaImage::from_fn(width, height, |_, _| WHITE),
        }
    }

    fn draw_house(&mut self, ctx: &RenderContext) {
        let house = if ctx.sun.is_between(ctx.instant, Sunset, Night)
            || ctx.sun.is_between(ctx.instant, NightEnd, Sunrise)
        {
            // It's dark outside, lights on.
            sprite("house_01")
        } else {
            // It's either day time or late at night, lights out in any case.
            sprite("house_00")
        };

        let y = ctx.temperature_to_y(ctx.data.current.air_temperature) - house.height() as i64;

        house.overlay(&mut self.img, 0, y);
    }

    fn draw_celestial_bodies(&mut self, ctx: &RenderContext) {
        let sun = sprite("sun_00");
        let next_sunrise = ctx.sun.next_phase(ctx.instant, Sunrise);
        let sun_x = ctx.timestamp_to_x(next_sunrise) - (sun.width() / 2) as i64;

        sun.overlay(&mut self.img, sun_x, 0);

        let moon = sprite("moon_00");
        let next_sunset = ctx.sun.next_phase(ctx.instant, Sunset);
        let moon_x = ctx.timestamp_to_x(next_sunset) - (moon.width() / 4) as i64;

        moon.overlay(&mut self.img, moon_x, 0);
    }

    fn draw_midday_and_midnight(&mut self, ctx: &RenderContext, line_points: &IndexMap<i64, i64>) {
        self.draw_flower(ctx, "flower_00", 0, line_points);
        self.draw_flower(ctx, "flower_01", 12, line_points);
    }

    fn draw_flower(
        &mut self,
        ctx: &RenderContext,
        name: &str,
        hour: i8,
        line_points: &IndexMap<i64, i64>,
    ) {
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

        if let Some(&y) = line_points.get(&x) {
            let sprite = sprite(name);
            let y = y - sprite.height() as i64;
            sprite.overlay(&mut self.img, x, y);
        }
    }

    fn draw_current_weather(&mut self, ctx: &RenderContext, line_points: &IndexMap<i64, i64>) {
        let weather = &ctx.data.current;
        let cloud_height = sprite("cloud_02").height() as i64;

        self.draw_house(ctx);
        self.draw_clouds(weather.cloud_area_fraction, 0, 5, ctx.x_offset);
        self.draw_fog(weather, 0, cloud_height + 10, ctx.x_offset, line_points);
        self.draw_precipitation(weather, 0, cloud_height + 5, ctx.x_offset, line_points);
        self.draw_temperature(ctx, weather.air_temperature, ctx.x_offset / 2);
    }

    fn draw_forecasts(&mut self, ctx: &RenderContext, line_points: &IndexMap<i64, i64>) {
        let forecasts = &ctx.data.forecasts;
        let cloud_height = sprite("cloud_02").height() as i64;

        // Only draw a forecast sample for every 4 hours. It'll get too crowded otherwise.
        for (i, forecast) in forecasts.iter().enumerate().step_by(4) {
            let x = ctx.forecast_x(i);
            self.draw_clouds(forecast.cloud_area_fraction, x, 5, ctx.x_step * 4);
            self.draw_trees(forecast, x, line_points);
            self.draw_fog(forecast, x, cloud_height + 10, ctx.x_step * 4, line_points);
            self.draw_precipitation(forecast, x, cloud_height + 5, ctx.x_step * 4, line_points);
        }

        self.draw_temperature_extrema(ctx, ctx.min_temperature);
        self.draw_temperature_extrema(ctx, ctx.max_temperature);
    }

    fn draw_temperature_extrema(&mut self, ctx: &RenderContext, temperature: f64) {
        if let Some((i, data_point)) = ctx
            .data
            .forecasts
            .iter()
            .enumerate()
            .find(|(_, dp)| dp.air_temperature == temperature)
        {
            let x = ctx.forecast_x(i);
            self.draw_temperature(ctx, data_point.air_temperature, x);
        }
    }

    fn draw_temperature(&mut self, ctx: &RenderContext, temperature: f64, x: i64) {
        let y = ctx.temperature_to_y(temperature);
        self.draw_digits(x, y + 5, temperature.round() as i64);
    }

    fn draw_clouds(&mut self, percentage: f64, x: i64, y: i64, width: i64) {
        let cloudset: &[usize] = match percentage {
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
            spriten("cloud", n).overlay(&mut self.img, x + offset, y);
        }
    }

    fn draw_fog(
        &mut self,
        data: &DataPoint,
        x: i64,
        y: i64,
        width: i64,
        line_points: &IndexMap<i64, i64>,
    ) {
        let x_max = x + width;
        let Some(&y_max) = (x..x_max).filter_map(|x| line_points.get(&x)).min() else {
            return;
        };

        let fog_width = width / 2;
        let y_step = 6;
        let y_range = (y_max - y) / 2;
        let mut rng = rand::thread_rng();

        for y_off in (0..y_range).step_by(y_step) {
            let y_pos = y + y_off;
            let percentage = (y_off as f64 / y_range as f64) * 100.0;

            if data.fog_area_fraction <= percentage {
                break;
            }

            let mut x_pos = x;

            loop {
                x_pos += rng.gen_range(3..fog_width / 2);

                if x_pos + fog_width > x_max {
                    break;
                }

                for i in 0..=fog_width {
                    let x = x_pos + i;
                    let y = y_pos + (i as f64 + 2.0).sin().round() as i64;

                    self.draw_pixel(x, y);
                }

                x_pos += fog_width;
            }
        }
    }

    fn draw_precipitation(
        &mut self,
        data: &DataPoint,
        x: i64,
        y: i64,
        width: i64,
        line_points: &IndexMap<i64, i64>,
    ) {
        if data.precipitation_amount <= 0.0 {
            // There's nothing that could fall from the sky.
            return;
        }

        let (heaviness, factor) = match data.condition {
            Condition::Snow => (HEAVY_SNOW, SNOW_FACTOR),
            Condition::Sleet => (HEAVY_SLEET, SLEET_FACTOR),
            _ => (HEAVY_RAIN, RAIN_FACTOR),
        };

        let r = 1.0 - (data.precipitation_amount / heaviness) / factor;

        for x in x..x + width {
            if let Some(&y_max) = line_points.get(&x) {
                for y in (y..y_max).step_by(2) {
                    if rand::random::<f64>() > r {
                        self.draw_pixel(x, y);

                        if let Condition::Snow = data.condition {
                            self.draw_pixel(x, y - 1);
                        } else if let Condition::Sleet = data.condition {
                            if rand::random() {
                                self.draw_pixel(x, y - 1);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_trees(&mut self, data: &DataPoint, x: i64, line_points: &IndexMap<i64, i64>) {
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

            if offset > line_points.len() as i64 {
                break;
            }

            if let Some(name) = trees.get(tree_index) {
                let y = line_points.get(&offset).unwrap();
                let tree = spriten(name, wind_index);
                let y_offset = (y - tree.height() as i64) + 1;
                tree.overlay(&mut self.img, x_offset, y_offset);
            }

            x_offset += 9;
        }
    }

    fn draw_digits(&mut self, x: i64, y: i64, value: i64) {
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

        sign.overlay(&mut self.img, x + offset, y);
        offset += digit_width + 1;

        if d1 > 0 {
            spriten("digit", d1 as _).overlay(&mut self.img, x + offset, y);
            offset += digit_width + 1;
        }

        spriten("digit", d2 as _).overlay(&mut self.img, x + offset, y);
    }

    fn draw_pixel(&mut self, x: i64, y: i64) {
        if x >= 0 && x < self.width() as i64 && y >= 0 && y < self.height() as i64 {
            trace!("drawing pixel at ({x}, {y})");
            self.img.put_pixel(x as u32, y as u32, BLACK);
        }
    }

    pub fn epd_bytes(&self) -> Result<Vec<u8>> {
        // The image needs to be rotated for the e-paper display.
        let image = imageops::rotate90(&self.img);
        let buf_len = buffer_len(WIDTH as usize, HEIGHT as usize);
        let mut buf = vec![Color::White.get_byte_value(); buf_len];
        let mut display =
            VarDisplay::new(WIDTH, HEIGHT, &mut buf, false).map_err(|err| anyhow!("{err:?}"))?;

        for (x, y, pixel) in image.enumerate_pixels() {
            let point = Point::new(x as i32, y as i32);

            if *pixel == BLACK {
                display.set_pixel(Pixel(point, Color::Black));
            } else {
                display.set_pixel(Pixel(point, Color::White));
            }
        }

        Ok(buf)
    }

    pub fn bmp_bytes(&self) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        self.img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp)?;
        Ok(buf)
    }
}

impl Deref for Canvas {
    type Target = RgbaImage;

    fn deref(&self) -> &Self::Target {
        &self.img
    }
}

impl DerefMut for Canvas {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.img
    }
}

#[derive(Debug)]
struct RenderContext<'a> {
    data: &'a WeatherData,
    sun: Sun,
    width: u32,
    // X-offset for the weather graph.
    x_offset: i64,
    // X-step for a single forecast.
    x_step: i64,
    // Y-offset for the weather graph.
    y_offset: i64,
    // The minimum temperature from the forecast.
    min_temperature: f64,
    // The maximum temperature from the forecast.
    max_temperature: f64,
    // Controls how many pixels to render per degree celsius.
    degrees_per_pixel: f64,
    // The instant at which the render context was created.
    instant: Timestamp,
}

impl<'a> RenderContext<'a> {
    fn create(data: &'a WeatherData, width: u32, height: u32) -> Result<Self> {
        let (latitude, longitude) = (data.coords.latitude, data.coords.longitude);
        let x_offset = sprite("house_00").width() as i64;
        let x_step = (width as i64 - x_offset) / (data.forecasts.len() as i64 - 1);
        let y_step = (height as f64 * 0.39).round() as i64;
        let y_offset = (height as i64 / 2) + y_step;
        let instant = Timestamp::now();

        let sun = Sun::new(latitude, longitude);

        let temperatures: Vec<f64> = data
            .forecasts
            .iter()
            // We'll ignore the last forecast in the temperature calculation because it's going to
            // be off-screen and is only used to draw the temperature line to the edge of the
            // screen.
            .take(data.forecasts.len() - 1)
            .map(|fc| fc.air_temperature)
            .collect();

        if temperatures.is_empty() {
            return Err(Error::new("forecast misses temperature data"));
        }

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

        Ok(RenderContext {
            data,
            sun,
            width,
            x_step,
            x_offset,
            y_offset,
            min_temperature,
            max_temperature,
            degrees_per_pixel,
            instant,
        })
    }

    fn timestamp_to_x(&self, timestamp: Timestamp) -> i64 {
        let delta = timestamp.duration_since(self.instant).as_secs_f64();
        let width = self.width as f64 - self.x_offset as f64;
        ((delta / SECONDS_DAY) * width).round() as i64 + self.x_offset
    }

    fn temperature_to_y(&self, temperature: f64) -> i64 {
        let delta = temperature - self.min_temperature;
        self.y_offset - (delta / self.degrees_per_pixel).round() as i64
    }

    fn forecast_x(&self, i: usize) -> i64 {
        self.x_offset + (self.x_step * (i as i64 + 1))
    }

    fn compute_line_points(&self) -> IndexMap<i64, i64> {
        // @FIXME(mohmann): now that we're at 24 forecasts data points again, it's probably enough
        // to just connect the dots with lines instead of having all this curve fitting code
        // around.
        let forecasts = &self.data.forecasts;
        let mut points: Vec<Coord2> = Vec::with_capacity(forecasts.len() + self.x_offset as usize);

        let y = self.temperature_to_y(self.data.current.air_temperature);

        // Points for the line below the house.
        for x in 0..self.x_offset {
            points.push(Coord2(x as f64, y as f64));
        }

        // Points for the temperatures.
        for (i, forecast) in forecasts.iter().enumerate() {
            let x = self.forecast_x(i);
            let y = self.temperature_to_y(forecast.air_temperature);
            points.push(Coord2(x as f64, y as f64));
        }

        // The heavy lifting.
        fit_curve_to_points(&points, 0.1).into_iter().collect()
    }
}
