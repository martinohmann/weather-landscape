mod sprites;

use self::sprites::sprite;
use crate::{error::Result, weather::Forecast};
use anyhow::anyhow;
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    color::Color,
    epd2in9_v2::{HEIGHT, WIDTH},
    graphics::VarDisplay,
};
use image::{imageops, ImageFormat, Rgba, RgbaImage};
use jiff::Timestamp;
use log::debug;
use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
};

const SECONDS_DAY: f64 = 24.0 * 60.0 * 60.0;
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

pub fn render(forecast: &Forecast) -> Result<Canvas> {
    debug!("rendering weather forecast {forecast:?}");

    // We'll flip width and height here. The e-paper display works in portrait mode but we'd like
    // to draw the image in landscape mode, because it's more intiutive. The rendered image gets
    // rotated by 90 degrees before serving it to the esp32.
    let mut canvas = Canvas::new(HEIGHT, WIDTH);

    canvas.draw_sun_and_moon(forecast);

    // Just some randomly placed sprites for now.
    sprite("house_00").overlay(&mut canvas, 10, 80);
    sprite("digit_01").overlay(&mut canvas, 1, 100);
    sprite("digit_00").overlay(&mut canvas, 5, 100);
    sprite("cloud_10").overlay(&mut canvas, 190, 0);
    sprite("flower_00").overlay(&mut canvas, 130, 80);
    sprite("tree_03").overlay(&mut canvas, 170, 80);

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

    fn ts_to_col(&self, start: Timestamp, timestamp: Timestamp) -> i64 {
        let delta = timestamp.duration_since(start).as_secs_f64();
        let width = self.width() as f64;
        ((delta / SECONDS_DAY) * width).round() as i64
    }

    fn draw_sun_and_moon(&mut self, forecast: &Forecast) {
        let sun = sprite("sun_00");
        let moon = sprite("moon_00");

        let sun_width = sun.width() as i64;
        let moon_width = moon.width() as i64;
        let max_width = self.img.width() as i64;

        // Ensure sun and moon are not partially outside of the canvas by clamping their position.
        let sun_x = self
            .ts_to_col(forecast.timestamp, forecast.next_sunrise)
            .clamp(0, max_width - sun_width);
        let moon_x = self
            .ts_to_col(forecast.timestamp, forecast.next_sunset)
            .clamp(0, max_width - moon_width);

        debug!("placing sun at ({sun_x},0)");
        sun.overlay(&mut self.img, sun_x, 0);

        debug!("placing moon ({moon_x},0)");
        moon.overlay(&mut self.img, moon_x, 0);
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
