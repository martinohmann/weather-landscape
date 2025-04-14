use super::{BLACK, TRANSPARENT, WHITE};
use crate::error::Result;
use image::{RgbaImage, imageops};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::OnceLock;
use tracing::trace;

macro_rules! load_sprite {
    ($map:ident, $name:literal) => {
        $map.insert(
            $name,
            Sprite::from_slice(
                $name,
                include_bytes!(concat!("data/sprites/", $name, ".png")),
            )
            .unwrap(),
        );
    };
}

pub(super) fn sprites() -> &'static HashMap<&'static str, Sprite> {
    static SPRITES: OnceLock<HashMap<&str, Sprite>> = OnceLock::new();

    SPRITES.get_or_init(|| {
        let mut m = HashMap::new();
        // Cloud
        load_sprite!(m, "cloud_02");
        load_sprite!(m, "cloud_03");
        load_sprite!(m, "cloud_05");
        load_sprite!(m, "cloud_10");
        load_sprite!(m, "cloud_30");
        load_sprite!(m, "cloud_50");
        // Digit
        load_sprite!(m, "digit_00");
        load_sprite!(m, "digit_01");
        load_sprite!(m, "digit_02");
        load_sprite!(m, "digit_03");
        load_sprite!(m, "digit_04");
        load_sprite!(m, "digit_05");
        load_sprite!(m, "digit_06");
        load_sprite!(m, "digit_07");
        load_sprite!(m, "digit_08");
        load_sprite!(m, "digit_09");
        load_sprite!(m, "digit_10");
        load_sprite!(m, "digit_11");
        load_sprite!(m, "digit_12");
        // East
        load_sprite!(m, "east_00");
        load_sprite!(m, "east_01");
        load_sprite!(m, "east_02");
        load_sprite!(m, "east_03");
        // Flower
        load_sprite!(m, "flower_00");
        load_sprite!(m, "flower_01");
        // House
        load_sprite!(m, "house_00");
        load_sprite!(m, "house_01");
        load_sprite!(m, "house_02");
        // Moon
        load_sprite!(m, "moon_00");
        load_sprite!(m, "moon_01");
        // Palm
        load_sprite!(m, "palm_00");
        load_sprite!(m, "palm_01");
        load_sprite!(m, "palm_02");
        load_sprite!(m, "palm_03");
        // Pine
        load_sprite!(m, "pine_00");
        load_sprite!(m, "pine_01");
        load_sprite!(m, "pine_02");
        load_sprite!(m, "pine_03");
        // Sun
        load_sprite!(m, "sun_00");
        // Temp
        load_sprite!(m, "temp_00");
        // Tree
        load_sprite!(m, "tree_00");
        load_sprite!(m, "tree_01");
        load_sprite!(m, "tree_02");
        load_sprite!(m, "tree_03");
        // Lightning
        load_sprite!(m, "lightning_00");
        load_sprite!(m, "lightning_01");
        load_sprite!(m, "lightning_02");
        load_sprite!(m, "lightning_03");
        load_sprite!(m, "lightning_04");
        load_sprite!(m, "lightning_05");
        m
    })
}

pub(super) fn sprite(name: &str) -> &'static Sprite {
    sprites().get(name).unwrap()
}

pub(super) fn spriten(prefix: &str, n: usize) -> &'static Sprite {
    let name = format!("{prefix}_{n:02}");
    sprite(&name)
}

#[derive(Debug)]
pub(super) struct Sprite {
    name: &'static str,
    img: RgbaImage,
}

impl Sprite {
    fn from_slice(name: &'static str, buf: &[u8]) -> Result<Self> {
        let mut img = image::load_from_memory(buf)?.into_rgba8();

        // Make any non-black, non-white pixels transparent.
        for pixel in img.pixels_mut() {
            if *pixel != BLACK && *pixel != WHITE {
                *pixel = TRANSPARENT;
            }
        }

        Ok(Sprite { name, img })
    }

    pub(super) fn name(&self) -> &'static str {
        self.name
    }

    pub(super) fn overlay(&self, image: &mut RgbaImage, x: i64, y: i64) {
        trace!("placing sprite {} at ({x}, {y})", self.name);
        imageops::overlay(image, &self.img, x, y);
    }
}

impl Deref for Sprite {
    type Target = RgbaImage;

    fn deref(&self) -> &Self::Target {
        &self.img
    }
}
