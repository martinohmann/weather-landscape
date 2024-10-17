#![allow(dead_code)]

mod config;
mod error;
mod graphics;
mod sun;
mod weather;

use actix_web::{
    get, http::header::ContentType, middleware, web::Data, App, HttpResponse, HttpServer, Result,
};
use config::AppConfig;
use weather::Weather;

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[get("/image.bmp")]
async fn image_bmp(weather: Data<Weather>) -> Result<HttpResponse> {
    let data = weather.get().await?;
    let image = graphics::render(&data)?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime::IMAGE_BMP))
        .body(image.bmp_bytes()?))
}

#[get("/image.epd")]
async fn image_epd(weather: Data<Weather>) -> Result<HttpResponse> {
    let data = weather.get().await?;
    let image = graphics::render(&data)?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(image.epd_bytes()?))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = AppConfig::load().await?;
    let weather = Weather::new(config.latitude, config.longitude)?;

    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(config.clone()))
            .app_data(Data::new(weather.clone()))
            .service(image_bmp)
            .service(image_epd)
            .service(healthz)
            .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .workers(2)
    .run()
    .await?;

    Ok(())
}
