mod wifi;

use anyhow::{bail, Result};
use esp_idf_hal::gpio::PinDriver;
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

    loop {
        led_pin.set_high()?;
        thread::sleep(Duration::from_secs(10));

        info!("Hello, world!");

        led_pin.set_low()?;
        thread::sleep(Duration::from_millis(300));
    }
}
