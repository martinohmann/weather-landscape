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
use esp_idf_hal::spi::{self, SpiDeviceDriver, SpiDriverConfig};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use log::info;
use std::thread;
use std::time::Duration;
use wifi::connect_wifi;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    let mut led_pin = PinDriver::output(peripherals.pins.gpio2)?;

    if let Err(err) = connect_wifi(
        CONFIG.wifi_ssid,
        CONFIG.wifi_psk,
        peripherals.modem,
        sysloop,
    ) {
        led_pin.set_high()?;
        bail!("Could not connect to Wi-Fi network: {:?}", err)
    }

    info!("Configuring the E-Ink display...");
    let mut display = Display2in9::default();

    let spi = peripherals.spi2;

    let sclk = peripherals.pins.gpio18;
    let serial_out = peripherals.pins.gpio23;
    let _cs = PinDriver::output(peripherals.pins.gpio5)?;
    let busy_in = PinDriver::input(peripherals.pins.gpio14)?;
    let dc = PinDriver::output(peripherals.pins.gpio13)?;
    let rst = PinDriver::output(peripherals.pins.gpio12)?;

    let config = spi::config::Config::new().baudrate(112500.into());
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

    thread::sleep(Duration::from_secs(3));
    let mut epd = Epd2in9::new(&mut device, busy_in, dc, rst, &mut delay, None)?;
    info!("E-Ink display init completed!");

    loop {
        info!("Paint it black");
        display.clear(Color::Black)?;
        epd.update_frame(&mut device, display.buffer(), &mut delay)?;
        epd.display_frame(&mut device, &mut delay)?;

        thread::sleep(Duration::from_secs(3));

        info!("Make it white");
        display.clear(Color::White)?;
        epd.update_frame(&mut device, display.buffer(), &mut delay)?;
        epd.display_frame(&mut device, &mut delay)?;

        thread::sleep(Duration::from_secs(3));

        info!("Draw the text");
        draw_text(&mut display, "PEBKAC!", 0, 0);
        epd.update_frame(&mut device, display.buffer(), &mut delay)?;
        epd.display_frame(&mut device, &mut delay)?;

        led_pin.set_high()?;
        thread::sleep(Duration::from_secs(3));

        info!("Hello, world!");

        led_pin.set_low()?;
        thread::sleep(Duration::from_millis(300));
    }
}

fn draw_text(display: &mut Display2in9, text: &str, x: i32, y: i32) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(Color::White)
        .background_color(Color::Black)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style(text, Point::new(x, y), style, text_style).draw(display);
}
