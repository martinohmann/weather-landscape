mod http;
mod wifi;

use anyhow::{Context, Result};
use embedded_graphics::prelude::*;
use epd_waveshare::{
    buffer_len,
    epd2in9_v2::{Display2in9, Epd2in9, HEIGHT, WIDTH},
    prelude::{Color, WaveshareDisplay},
};
use esp_idf_hal::{
    delay::Ets,
    gpio::{AnyIOPin, Gpio2, PinDriver},
    prelude::*,
    spi::{SpiDeviceDriver, SpiDriverConfig, config::Config as SpiConfig},
};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::{error, info};
use std::{thread, time::Duration};

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default(10)]
    deep_sleep_seconds: u64,
    #[default(0)]
    clear_after_seconds: u64,
    #[default("")]
    data_url: &'static str,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    if let Err(err) = run(peripherals, sysloop, nvs) {
        error!("{err}");
    }

    enter_deep_sleep(Duration::from_secs(CONFIG.deep_sleep_seconds));
}

fn run(
    peripherals: Peripherals,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<()> {
    let wifi = wifi::connect(
        CONFIG.wifi_ssid,
        CONFIG.wifi_psk,
        peripherals.modem,
        sysloop,
        nvs,
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

    info!("Drawing image");
    epd.update_and_display_frame(&mut spi, &image_data, &mut delay)?;

    #[allow(clippy::absurd_extreme_comparisons)]
    if CONFIG.clear_after_seconds > 0 {
        thread::sleep(Duration::from_secs(CONFIG.clear_after_seconds));

        info!("Clearing display");
        let mut display = Display2in9::default();
        display.clear(Color::White)?;
        epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;
    }

    epd.sleep(&mut spi, &mut delay)?;
    Ok(())
}

fn enter_deep_sleep(sleep_time: Duration) -> ! {
    info!("Entering deep sleep");
    unsafe { esp_idf_sys::esp_deep_sleep(sleep_time.as_micros() as u64) }
}

const fn display_buffer_size() -> usize {
    buffer_len(WIDTH as usize, HEIGHT as usize)
}
