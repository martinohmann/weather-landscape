mod sprites;

use self::sprites::sprite;
use crate::{
    error::{Error, Result},
    weather::Forecast,
};
use anyhow::anyhow;
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    color::Color,
    epd2in9_v2::{HEIGHT, WIDTH},
    graphics::VarDisplay,
};
use image::{imageops, ImageFormat, Rgba, RgbaImage};
use imageproc::drawing::draw_line_segment_mut;
use jiff::Timestamp;
use log::debug;
use std::{
    cmp::Ordering,
    io::Cursor,
    ops::{Deref, DerefMut},
};

const SECONDS_DAY: f64 = 24.0 * 60.0 * 60.0;
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);
const X_OFFSET_DEFAULT: i64 = 32;

pub fn render(forecast: &Forecast) -> Result<Canvas> {
    // We'll flip width and height here. The e-paper display works in portrait mode but we'd like
    // to draw the image in landscape mode, because it's more intiutive. The rendered image gets
    // rotated by 90 degrees before serving it to the esp32.
    let mut canvas = Canvas::new(HEIGHT, WIDTH);
    let ctx = RenderContext::create(forecast, canvas.width(), canvas.height())?;

    debug!("rendering context {ctx:?}");

    canvas.draw_house(&ctx);
    canvas.draw_sun_and_moon(&ctx);
    canvas.draw_temperature(&ctx);

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
        let house = sprite("house_00");
        let y = ctx.degrees_to_y(ctx.current_temperature);
        let house_y = y - house.height() as i64;

        debug!("placing house at (0, {house_y})");
        house.overlay(&mut self.img, 0, house_y);

        debug!(
            "drawing current temperature line from (0, {y}) to ({}, {y})",
            ctx.x_offset - 1
        );
        for x in 0..ctx.x_offset {
            self.draw_pixel(x, y);
        }

        self.draw_digits(
            ctx.x_offset / 2,
            y + 5,
            ctx.current_temperature.round() as i64,
        );
    }

    fn draw_sun_and_moon(&mut self, ctx: &RenderContext) {
        let sun = sprite("sun_00");
        let moon = sprite("moon_00");
        let sun_x = ctx.timestamp_to_x(ctx.forecast.next_sunrise, 0) - (sun.width() / 2) as i64;
        let moon_x = ctx.timestamp_to_x(ctx.forecast.next_sunset, 0) - (moon.width() / 4) as i64;

        debug!("placing sun at ({sun_x},0)");
        sun.overlay(&mut self.img, sun_x, 0);

        debug!("placing moon at ({moon_x},0)");
        moon.overlay(&mut self.img, moon_x, 0);
    }

    fn draw_temperature(&mut self, ctx: &RenderContext) {
        let num_forecasts = ctx.forecast.hourly_forecast.len() - 1;
        let x_step = (self.width() as i64 - ctx.x_offset) / num_forecasts as i64;

        let mut x = ctx.x_offset;
        let mut points: Vec<(f32, f32)> = Vec::new();
        let mut max_temperature_drawn = false;
        let mut min_temperature_drawn = false;

        for hourly_forecast in ctx.forecast.hourly_forecast.iter() {
            let temperature = hourly_forecast.air_temperature;
            let y = ctx.degrees_to_y(temperature);

            debug!("drawing temperature {temperature} at ({x}, {y})");
            self.draw_pixel(x, y);

            if temperature == ctx.max_temperature && !max_temperature_drawn {
                self.draw_digits(x, y + 5, temperature.round() as i64);
                max_temperature_drawn = true;
            } else if temperature == ctx.min_temperature && !min_temperature_drawn {
                self.draw_digits(x, y + 5, temperature.round() as i64);
                min_temperature_drawn = true;
            }

            points.push((x as f32, y as f32));

            x += x_step;
        }

        // Connect the temperature points with lines.
        // @TODO(mohmann): use bezier curves instead to make the landscape look nicer.
        for w in points.windows(2) {
            let p0 = w[0];
            let p1 = w[1];
            debug!("drawing line from {p0:?} to {p1:?}");
            draw_line_segment_mut(&mut self.img, p0, p1, BLACK);
        }
    }

    fn draw_digits(&mut self, x: i64, y: i64, value: i64) {
        debug!("drawing digits for value {value} at ({x}, {y})");
        // @TODO(mohmann): draw actual digits.
        for i in 0..5 {
            self.draw_pixel(x, y + i);
        }
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
    forecast: &'a Forecast,
    width: u32,
    height: u32,
    // X-axis offset for the weather graph.
    x_offset: i64,
    // Y-axis offset for the weather graph.
    y_offset: i64,
    y_step: i64,
    // The current temperature from the forecast.
    current_temperature: f64,
    min_temperature: f64,
    max_temperature: f64,
    temperature_range: f64,
    // Controls how many pixels to render per degree celsius.
    degrees_per_pixel: f64,
}

impl<'a> RenderContext<'a> {
    fn create(forecast: &'a Forecast, width: u32, height: u32) -> Result<Self> {
        let y_step = (height as f64 * 0.39).round() as i64;
        let y_offset = (height / 2) as i64;

        let temperatures: Vec<f64> = forecast
            .hourly_forecast
            .iter()
            // We'll ignore the last forecast in the temperature calculation because it's going to
            // be off-screen and is only used to draw the temperature line to the edge of the
            // screen.
            .take(forecast.hourly_forecast.len() - 1)
            .map(|fc| fc.air_temperature)
            .collect();

        if temperatures.is_empty() {
            return Err(Error::new("forecast misses temperature data"));
        }

        let min_temperature = *temperatures
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap();
        let max_temperature = *temperatures
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap();
        let current_temperature = temperatures[0];
        let temperature_range = max_temperature - min_temperature;

        let degrees_per_pixel = if temperature_range < y_step as f64 {
            0.5
        } else {
            temperature_range / y_step as f64
        };

        Ok(RenderContext {
            forecast,
            width,
            height,
            x_offset: X_OFFSET_DEFAULT,
            y_step,
            y_offset,
            current_temperature,
            min_temperature,
            max_temperature,
            temperature_range,
            degrees_per_pixel,
        })
    }

    fn timestamp_to_x(&self, timestamp: Timestamp, x_offset: i64) -> i64 {
        let delta = timestamp
            .duration_since(self.forecast.timestamp)
            .as_secs_f64();
        let width = self.width as f64 - x_offset as f64;
        ((delta / SECONDS_DAY) * width).round() as i64 + x_offset
    }

    fn degrees_to_y(&self, temperature: f64) -> i64 {
        let n = ((temperature - self.min_temperature) / self.degrees_per_pixel).round() as i64;
        self.y_offset + self.y_step - n
    }
}
