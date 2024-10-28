use super::display_buffer_size;
use anyhow::{bail, Result};
use embedded_svc::{
    http::{client::Client, Method},
    utils::io,
};
use esp_idf_svc::{
    http::client::{Configuration, EspHttpConnection},
    sys::esp_crt_bundle_attach,
};
use log::info;
use std::time::Duration;

pub fn fetch_image_data(url: &str) -> Result<Vec<u8>> {
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

    let mut buf = vec![0; display_buffer_size()];
    let len = io::try_read_full(response, &mut buf).map_err(|err| err.0)?;

    info!("Received {len} bytes");

    Ok(buf[..len].to_vec())
}
