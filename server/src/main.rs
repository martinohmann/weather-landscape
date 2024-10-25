mod config;
mod error;
mod graphics;
mod sun;
mod weather;

use actix_web::{
    get,
    http::header::ContentType,
    middleware,
    web::{Data, Path, Query},
    App, HttpResponse, HttpServer, Result,
};
use config::Config;
use serde::Deserialize;
use weather::Weather;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum ImageFormat {
    /// Bitmap
    Bmp,
    /// E-paper display
    Epd,
}

#[derive(Deserialize, Debug)]
struct ImageRequest {
    #[serde(default)]
    cause_havoc: bool,
}

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[get("/image.{format}")]
async fn image(
    config: Data<Config>,
    weather: Data<Weather>,
    format: Path<ImageFormat>,
    query: Query<ImageRequest>,
) -> Result<HttpResponse> {
    let mut data = weather.get().await?;

    if query.cause_havoc {
        weather::cause_havoc(&mut data);
    }

    let image = graphics::render(&config, &data)?;

    let (content_type, body) = match format.into_inner() {
        ImageFormat::Bmp => (mime::IMAGE_BMP, image.bmp_bytes()?),
        ImageFormat::Epd => (mime::APPLICATION_OCTET_STREAM, image.epd_bytes()?),
    };

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(content_type))
        .body(body))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = Config::load().await?;
    let weather = Weather::new(config.latitude, config.longitude)?;

    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(config.clone()))
            .app_data(Data::new(weather.clone()))
            .service(image)
            .service(healthz)
            .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .workers(2)
    .run()
    .await?;

    Ok(())
}
