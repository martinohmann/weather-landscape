mod http;
mod wifi;

use anyhow::{bail, Result};
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    prelude::{DrawTarget, Point},
    text::{Baseline, Text, TextStyleBuilder},
    Drawable,
};
use epd_waveshare::{
    epd2in9_v2::{Display2in9, Epd2in9},
    prelude::{Color, WaveshareDisplay},
};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{AnyIOPin, Gpio2, PinDriver};
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{self, SpiDeviceDriver, SpiDriverConfig};
use esp_idf_svc::eventloop::EspSystemEventLoop;
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
    #[default("")]
    data_url: &'static str,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

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

    if let Err(err) = http::get(CONFIG.data_url) {
        error!("Failed to request image data: {err}")
    }

    info!("Disconnecting Wifi");
    drop(wifi);

    info!("Configuring the E-Ink display...");
    let mut display = Display2in9::default();

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
    info!("E-Ink display init completed!");

    info!("Draw text");
    display.clear(Color::White)?;
    draw_text(&mut display, "PEBKAC!", 0, 0);
    epd.update_frame(&mut device, display.buffer(), &mut delay)?;
    epd.display_frame(&mut device, &mut delay)?;

    thread::sleep(Duration::from_secs(10));

    info!("Clearing display");
    display.clear(Color::White)?;
    epd.update_frame(&mut device, display.buffer(), &mut delay)?;
    epd.display_frame(&mut device, &mut delay)?;

    enter_deep_sleep(Duration::from_secs(CONFIG.deep_sleep_seconds));
}

fn draw_text(display: &mut Display2in9, text: &str, x: i32, y: i32) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(Color::Black)
        .background_color(Color::White)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style(text, Point::new(x, y), style, text_style).draw(display);
}

fn enter_deep_sleep(sleep_time: Duration) -> ! {
    info!("Entering deep sleep");
    unsafe { esp_idf_sys::esp_deep_sleep(sleep_time.as_micros() as u64) }
}
