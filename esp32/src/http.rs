use super::display_buffer_size;
use anyhow::{Result, bail};
use embedded_svc::{
    http::{Method, client::Client},
    utils::io,
};
use esp_idf_svc::{
    http::client::{Configuration, EspHttpConnection},
    sys::esp_crt_bundle_attach,
};
use log::info;
use std::time::Duration;

const HEADER_X_ESP_DEEP_SLEEP_SECONDS: &str = "x-esp-deep-sleep-seconds";

pub struct Response {
    pub image_data: Vec<u8>,
    pub deep_sleep_seconds: Option<u64>,
}

pub fn fetch_data(url: &str) -> Result<Response> {
    let connection = EspHttpConnection::new(&Configuration {
        timeout: Some(Duration::from_secs(5)),
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(connection);

    info!("Requesting {url}");

    let headers = [("accept", "application/octet-stream")];
    let response = client.request(Method::Get, url, &headers)?.submit()?;
    let status = response.status();

    if status != 200 {
        bail!("Expected response code 200, got {status}");
    }

    let deep_sleep_seconds = response
        .header(HEADER_X_ESP_DEEP_SLEEP_SECONDS)
        .and_then(|value| value.parse().ok());

    let mut buf = vec![0; display_buffer_size()];
    let len = io::try_read_full(response, &mut buf).map_err(|err| err.0)?;

    info!("Received {len} bytes");

    Ok(Response {
        image_data: buf[..len].to_vec(),
        deep_sleep_seconds,
    })
}
