mod http;
mod wifi;

use anyhow::{anyhow, bail, Result};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use epd_waveshare::{
    buffer_len,
    epd2in9_v2::{Display2in9, Epd2in9, HEIGHT, WIDTH},
    prelude::{Color, DisplayRotation, WaveshareDisplay},
};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{AnyIOPin, Gpio2, PinDriver};
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{self, SpiDeviceDriver, SpiDriverConfig};
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
    let mut led_pin = PinDriver::output(peripherals.pins.gpio2)?;

    let wifi = match wifi::connect(
        CONFIG.wifi_ssid,
        CONFIG.wifi_psk,
        peripherals.modem,
        sysloop,
    ) {
        Ok(wifi) => wifi,
        Err(err) => {
            led_pin.set_high()?;
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };

    let image_data = http::fetch_image_data(CONFIG.data_url)?;

    info!("Disconnecting Wifi");
    drop(wifi);

    info!("Configuring the E-Ink display...");
    let spi = peripherals.spi2;

    let sclk = peripherals.pins.gpio18;
    let serial_out = peripherals.pins.gpio23;
    let _cs = PinDriver::output(peripherals.pins.gpio5)?;
    let busy_in = PinDriver::input(peripherals.pins.gpio14)?;
    let dc = PinDriver::output(peripherals.pins.gpio13)?;
    let rst = PinDriver::output(peripherals.pins.gpio12)?;

    let config = spi::config::Config::new().baudrate(4_000_000.into());
    let mut device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        serial_out,
        Option::<Gpio2>::None,
        Option::<AnyIOPin>::None,
        &SpiDriverConfig::default(),
        &config,
    )?;

    let mut delay = Ets;

    let mut epd = Epd2in9::new(&mut device, busy_in, dc, rst, &mut delay, None)?;
    let mut display = Display2in9::default();
    info!("E-Ink display init completed!");

    let bmp = Bmp::<BinaryColor>::from_slice(&image_data)
        .map_err(|err| anyhow!("failed to parse BMP: {err:?}"))?;
    let bmp_header = bmp.as_raw().header();

    info!("Drawing image");

    if bmp_header.image_size.width > bmp_header.image_size.height {
        display.set_rotation(DisplayRotation::Rotate90);
    }

    display.draw_iter(
        bmp.pixels()
            .map(|Pixel(position, color)| Pixel(position, Color::from(color).inverse())),
    )?;
    epd.update_frame(&mut device, display.buffer(), &mut delay)?;
    epd.display_frame(&mut device, &mut delay)?;

    thread::sleep(Duration::from_secs(10));

    info!("Clearing display");
    display.clear(Color::White)?;
    epd.update_frame(&mut device, display.buffer(), &mut delay)?;
    epd.display_frame(&mut device, &mut delay)?;

    Ok(())
}

fn enter_deep_sleep(sleep_time: Duration) -> ! {
    info!("Entering deep sleep");
    unsafe { esp_idf_sys::esp_deep_sleep(sleep_time.as_micros() as u64) }
}

const fn display_buffer_size() -> usize {
    buffer_len(WIDTH as usize, HEIGHT as usize)
}
