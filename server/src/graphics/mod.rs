mod sprites;
mod util;

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
use log::debug;
use std::io::Cursor;
use util::timestamp_to_column;

const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

pub struct Renderer {}

impl Renderer {
    pub fn new() -> Self {
        Renderer {}
    }

    pub fn render_image(&self, forecast: &Forecast) -> Result<Image> {
        debug!("rendering weather forecast {forecast:?}");

        let mut image = RgbaImage::from_fn(HEIGHT, WIDTH, |_, _| WHITE);

        let sunrise_col = timestamp_to_column(forecast.timestamp, forecast.next_sunrise);
        let sunset_col = timestamp_to_column(forecast.timestamp, forecast.next_sunset);

        debug!("placing sunrise at {sunrise_col} and sunset at {sunset_col}");

        sprite("moon_00").overlay(&mut image, sunset_col.min(HEIGHT as i64 - 30), 0);
        sprite("sun_00").overlay(&mut image, sunrise_col.min(HEIGHT as i64 - 30), 0);

        // Just some randomly placed sprites for now.
        sprite("house_00").overlay(&mut image, 10, 80);
        sprite("digit_01").overlay(&mut image, 1, 100);
        sprite("digit_00").overlay(&mut image, 5, 100);
        sprite("cloud_10").overlay(&mut image, 190, 0);
        sprite("flower_00").overlay(&mut image, 130, 80);
        sprite("tree_03").overlay(&mut image, 170, 80);

        Ok(Image(image))
    }
}

#[derive(Debug)]
pub struct Image(RgbaImage);

impl Image {
    pub fn epd_bytes(&self) -> Result<Vec<u8>> {
        // The image needs to be rotated for the e-paper display.
        let image = imageops::rotate90(&self.0);
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
        self.0
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp)?;
        Ok(buf)
    }
}
