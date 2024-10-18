mod curve;
mod sprites;

use self::curve::fit_curve_to_points;
use self::sprites::{sprite, spriten};
use crate::{
    error::{Error, Result},
    sun,
    weather::WeatherData,
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
use image::{imageops, ImageFormat, Rgba, RgbaImage};
use indexmap::IndexMap;
use jiff::civil::time;
use jiff::{SignedDuration, Timestamp, Zoned};
use log::debug;
use rand::Rng;
use sprites::Sprite;
use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
};

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

    debug!("rendering context {ctx:?}");

    let line_points = ctx.compute_line_points();

    debug!("{} line points: {:?}", line_points.len(), line_points);

    canvas.draw_house(&ctx);
    canvas.draw_sun_and_moon(&ctx);
    canvas.draw_clouds(ctx.data.current.cloud_area_fraction, 0, 5, ctx.x_offset);
    canvas.draw_forecasts(&ctx);
    canvas.draw_midday_and_midnight(&ctx, &line_points);
    canvas.draw_temperature_extrema(&ctx, ctx.min_temperature);
    canvas.draw_temperature_extrema(&ctx, ctx.max_temperature);

    for (x, y) in line_points {
        canvas.draw_pixel(x, y);
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
        let house = if ctx.now < ctx.next_sunrise && ctx.next_sunrise < ctx.next_sunset {
            sprite("house_01") // Night time, lights out!
        } else {
            sprite("house_00")
        };
        let current_temperature = ctx.data.current.air_temperature;
        let y = ctx.temperature_to_y(current_temperature);
        let house_y = y - house.height() as i64;

        debug!("placing house at (0, {house_y})");
        house.overlay(&mut self.img, 0, house_y);

        self.draw_digits(ctx.x_offset / 2, y + 5, current_temperature.round() as i64);
    }

    fn draw_sun_and_moon(&mut self, ctx: &RenderContext) {
        let sun = sprite("sun_00");
        let moon = sprite("moon_00");
        let sun_x = ctx.timestamp_to_x(ctx.next_sunrise) - (sun.width() / 2) as i64;
        let moon_x = ctx.timestamp_to_x(ctx.next_sunset) - (moon.width() / 4) as i64;

        debug!("placing sun at ({sun_x},0)");
        sun.overlay(&mut self.img, sun_x, 0);

        debug!("placing moon at ({moon_x},0)");
        moon.overlay(&mut self.img, moon_x, 0);
    }

    fn draw_midday_and_midnight(&mut self, ctx: &RenderContext, line_points: &IndexMap<i64, i64>) {
        self.draw_flower(ctx, line_points, sprite("flower_00"), 0);
        self.draw_flower(ctx, line_points, sprite("flower_01"), 12);
    }

    fn draw_flower(
        &mut self,
        ctx: &RenderContext,
        line_points: &IndexMap<i64, i64>,
        sprite: &Sprite,
        hour: i8,
    ) {
        let now = Zoned::now();
        let mut time = now.with().time(time(hour, 0, 0, 0)).build().unwrap();
        if time < now {
            time = time.checked_add(SignedDuration::from_hours(24)).unwrap();
        }

        let x = ctx.timestamp_to_x(time.timestamp());

        if x < ctx.x_offset {
            // We don't want it to overlap with the house, or do we?
            return;
        }

        if let Some(y) = line_points.get(&x) {
            let y = *y - sprite.height() as i64;
            sprite.overlay(&mut self.img, x, y);
        }
    }

    fn draw_forecasts(&mut self, ctx: &RenderContext) {
        let forecasts = &ctx.data.forecasts;

        // Only draw a forecast sample for every 4 hours. It'll get too crowded otherwise.
        for (i, forecast) in forecasts.iter().enumerate().step_by(4) {
            let x = ctx.forecast_x(i);
            self.draw_clouds(forecast.cloud_area_fraction, x, 5, ctx.x_step);
        }
    }

    fn draw_temperature_extremas(&mut self, ctx: &RenderContext) {
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
            let y = ctx.temperature_to_y(data_point.air_temperature);
            self.draw_digits(x, y + 5, temperature.round() as i64);
        }
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

    fn draw_digits(&mut self, x: i64, y: i64, value: i64) {
        debug!("drawing digits for value {value} at ({x}, {y})");

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
    width: u32,
    height: u32,
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
    now: Timestamp,
    next_sunrise: Timestamp,
    next_sunset: Timestamp,
}

impl<'a> RenderContext<'a> {
    fn create(data: &'a WeatherData, width: u32, height: u32) -> Result<Self> {
        let x_offset = sprite("house_00").width() as i64;
        let x_step = (width as i64 - x_offset) / (data.forecasts.len() as i64 - 1);
        let y_step = (height as f64 * 0.39).round() as i64;
        let y_offset = (height as i64 / 2) + y_step;
        let now = Timestamp::now();

        let next_sunrise = sun::next_sunrise(data.coords.latitude, data.coords.longitude, now)?;
        let next_sunset = sun::next_sunset(data.coords.latitude, data.coords.longitude, now)?;

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
            width,
            height,
            x_step,
            x_offset,
            y_offset,
            min_temperature,
            max_temperature,
            degrees_per_pixel,
            now,
            next_sunrise,
            next_sunset,
        })
    }

    fn timestamp_to_x(&self, timestamp: Timestamp) -> i64 {
        let delta = timestamp.duration_since(self.now).as_secs_f64();
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
