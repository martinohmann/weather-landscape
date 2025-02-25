use crate::error::Result;
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    color::Color,
    epd2in9_v2::{HEIGHT, WIDTH},
    graphics::VarDisplay,
};
use image::{Pixel, Rgba, RgbaImage, imageops};
use serde::Deserialize;
use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
};
use tracing::trace;

pub(super) const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
pub(super) const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
pub(super) const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

/// An image buffer that can be encoded in various formats.
#[derive(Debug)]
pub struct Image(RgbaImage);

impl Image {
    pub(super) fn new(width: u32, height: u32) -> Self {
        Image(RgbaImage::from_fn(width, height, |_, _| WHITE))
    }

    pub(super) fn draw_pixel(&mut self, x: i64, y: i64) {
        if x >= 0 && x < self.width() as i64 && y >= 0 && y < self.height() as i64 {
            trace!("drawing pixel at ({x}, {y})");
            self.0.put_pixel(x as u32, y as u32, BLACK);
        }
    }

    pub(super) fn invert_pixels(&mut self) {
        for pixel in self.pixels_mut() {
            pixel.invert();
        }
    }

    fn encode_epd(&self) -> Result<Vec<u8>> {
        // The image needs to be rotated for the e-paper display.
        let image = imageops::rotate90(&self.0);
        let buf_len = buffer_len(WIDTH as usize, HEIGHT as usize);
        let mut buf = vec![Color::White.get_byte_value(); buf_len];
        let mut display = VarDisplay::new(WIDTH, HEIGHT, &mut buf, false)?;

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

    fn encode_as(&self, format: image::ImageFormat) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        self.0.write_to(&mut Cursor::new(&mut buf), format)?;
        Ok(buf)
    }

    /// Encodes the image in given format, returning the encoded bytes and a MIME type suitable for
    /// serving the image.
    pub fn encode(&self, format: ImageFormat) -> Result<(Vec<u8>, mime::Mime)> {
        let bytes = match format {
            ImageFormat::Epd => self.encode_epd()?,
            ImageFormat::Png => self.encode_as(image::ImageFormat::Png)?,
            ImageFormat::Gif => self.encode_as(image::ImageFormat::Gif)?,
            ImageFormat::Bmp => self.encode_as(image::ImageFormat::Bmp)?,
        };
        Ok((bytes, format.mime_type()))
    }
}

impl Deref for Image {
    type Target = RgbaImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Image {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Supported image formats.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    /// Raw bytes for an E-paper display.
    Epd,
    /// PNG image.
    Png,
    /// GIF image.
    Gif,
    /// BMP image.
    Bmp,
}

impl ImageFormat {
    /// Returns a MIME type suitable for serving the encoded image bytes.
    pub fn mime_type(&self) -> mime::Mime {
        match self {
            ImageFormat::Epd => mime::APPLICATION_OCTET_STREAM,
            ImageFormat::Png => mime::IMAGE_PNG,
            ImageFormat::Gif => mime::IMAGE_GIF,
            ImageFormat::Bmp => mime::IMAGE_BMP,
        }
    }
}
