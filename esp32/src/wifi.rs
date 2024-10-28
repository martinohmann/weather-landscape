use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    hal::peripheral::Peripheral,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::info;

pub fn connect(
    ssid: &str,
    password: &str,
    modem: impl Peripheral<P = Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<BlockingWifi<EspWifi<'static>>> {
    let mut auth_method = AuthMethod::WPA2Personal;

    if ssid.is_empty() {
        bail!("Missing WiFi name")
    }

    if password.is_empty() {
        auth_method = AuthMethod::None;
        info!("WiFi password is empty");
    }

    let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    let config = Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .expect("Could not parse the given SSID into WiFi config"),
        password: password
            .try_into()
            .expect("Could not parse the given password into WiFi config"),
        auth_method,
        ..Default::default()
    });

    wifi.set_configuration(&config)?;

    info!("Starting WiFi...");
    wifi.start()?;

    info!("Connecting WiFi...");
    wifi.connect()?;

    info!("Waiting for DHCP lease...");
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("WiFi DHCP info: {ip_info:?}");

    Ok(wifi)
}
