use crate::error::Result;
use anyhow::anyhow;
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    color::Color,
    epd2in9_v2::{HEIGHT, WIDTH},
    graphics::VarDisplay,
};
use image::{imageops, ImageBuffer, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const WHITE: Rgb<u8> = Rgb([255, 255, 255]);

pub type Image = ImageBuffer<Rgb<u8>, Vec<u8>>;

pub struct Renderer {}

impl Renderer {
    pub fn new() -> Self {
        Renderer {}
    }

    pub fn render_image(&self) -> Result<Image> {
        // This is relatively stupid right now.
        let mut img = RgbImage::from_fn(HEIGHT, WIDTH, |x, y| {
            if x % 2 == 0 || y % 2 == 0 {
                BLACK
            } else {
                WHITE
            }
        });

        let overlay =
            image::load_from_memory(include_bytes!("../data/sprites/house_00.png"))?.into_rgb8();

        imageops::overlay(&mut img, &overlay, 10, 10);
        let img = imageops::rotate90(&img);

        Ok(img)
    }
}

pub fn epd_bytes(image: &Image) -> Result<Vec<u8>> {
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

pub fn bmp_bytes(image: &Image) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    image.write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp)?;
    Ok(buf)
}
