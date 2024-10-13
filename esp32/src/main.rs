mod http;
mod wifi;

use anyhow::{anyhow, bail, Context, Result};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use epd_waveshare::{
    buffer_len,
    epd2in9_v2::{Display2in9, Epd2in9, HEIGHT, WIDTH},
    prelude::{Color, DisplayRotation, WaveshareDisplay},
};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{AnyIOPin, Gpio2, PinDriver};
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{config::Config as SpiConfig, SpiDeviceDriver, SpiDriverConfig};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use log::{error, info};
use std::{thread, time::Duration};
use tinybmp::Bmp;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default(10)]
    deep_sleep_seconds: u64,
    #[default("")]
    data_url: &'static str,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    if let Err(err) = run(peripherals, sysloop) {
        error!("{err}");
    }

    enter_deep_sleep(Duration::from_secs(CONFIG.deep_sleep_seconds));
}

fn run(peripherals: Peripherals, sysloop: EspSystemEventLoop) -> Result<()> {
    let wifi = wifi::connect(
        CONFIG.wifi_ssid,
        CONFIG.wifi_psk,
        peripherals.modem,
        sysloop,
    )
    .context("Could not connect to WiFi network")?;

    let image_data = http::fetch_image_data(CONFIG.data_url)?;

    info!("Disconnecting WiFi");
    drop(wifi);

    info!("Configuring the E-Ink display...");
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio18;
    let serial_out = peripherals.pins.gpio23;

    let mut spi = SpiDeviceDriver::new_single(
        spi,
        sclk,
        serial_out,
        Option::<Gpio2>::None,
        Option::<AnyIOPin>::None,
        &SpiDriverConfig::default(),
        &SpiConfig::default(),
    )?;

    let _cs = PinDriver::output(peripherals.pins.gpio5)?;
    let busy_in = PinDriver::input(peripherals.pins.gpio14)?;
    let dc = PinDriver::output(peripherals.pins.gpio13)?;
    let rst = PinDriver::output(peripherals.pins.gpio12)?;
    let mut delay = Ets;

    let mut epd = Epd2in9::new(&mut spi, busy_in, dc, rst, &mut delay, None)?;
    info!("E-Ink display init completed!");

    let mut display = Display2in9::default();

    info!("Drawing image");
    draw_image(&mut display, &image_data)?;
    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;

    thread::sleep(Duration::from_secs(10));

    info!("Clearing display");
    display.clear(Color::White)?;
    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;

    epd.sleep(&mut spi, &mut delay)?;
    Ok(())
}

fn enter_deep_sleep(sleep_time: Duration) -> ! {
    info!("Entering deep sleep");
    unsafe { esp_idf_sys::esp_deep_sleep(sleep_time.as_micros() as u64) }
}

fn draw_image(display: &mut Display2in9, image_data: &[u8]) -> Result<()> {
    let bmp = Bmp::<BinaryColor>::from_slice(image_data)
        .map_err(|err| anyhow!("Failed to parse BMP: {err:?}"))?;
    let bmp_header = bmp.as_raw().header();

    if bmp_header.image_size.width > bmp_header.image_size.height {
        display.set_rotation(DisplayRotation::Rotate90);
    }

    display.draw_iter(
        bmp.pixels()
            .map(|Pixel(position, color)| Pixel(position, Color::from(color).inverse())),
    )?;

    Ok(())
}

const fn display_buffer_size() -> usize {
    buffer_len(WIDTH as usize, HEIGHT as usize)
}
