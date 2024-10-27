mod config;
mod error;
mod graphics;
mod sun;
mod weather;

use actix_web::{
    get,
    http::header::ContentType,
    middleware::Logger,
    web::{Data, Path, Query},
    App, HttpResponse, HttpServer, Result,
};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::anyhow;
use config::Config;
use prometheus::{opts, IntCounterVec};
use serde::Deserialize;
use weather::Weather;

const METRICS_NAMESPACE: &str = "landscape_weather_server";

#[derive(Deserialize, Debug, Clone, Copy)]
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
    counter: Data<IntCounterVec>,
    format: Path<ImageFormat>,
    query: Query<ImageRequest>,
) -> Result<HttpResponse> {
    let mut data = weather.get().await?;

    if query.cause_havoc {
        weather::cause_havoc(&mut data);
    }

    let image = graphics::render(&config, &data)?;

    let (mime_type, body) = match format.into_inner() {
        ImageFormat::Bmp => (mime::IMAGE_BMP, image.bmp_bytes()?),
        ImageFormat::Epd => (mime::APPLICATION_OCTET_STREAM, image.epd_bytes()?),
    };

    counter.with_label_values(&[mime_type.essence_str()]).inc();

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime_type))
        .body(body))
}

async fn run() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = Config::load().await?;
    let weather = Weather::new(config.latitude, config.longitude)?;

    let prometheus = PrometheusMetricsBuilder::new(METRICS_NAMESPACE)
        .endpoint("/metrics")
        .build()
        .map_err(|err| anyhow!("{err}"))?;

    let counter_opts = opts!("image_requests_total", "Total number of image requests")
        .namespace(METRICS_NAMESPACE);
    let counter = IntCounterVec::new(counter_opts, &["mime_type"])?;

    prometheus.registry.register(Box::new(counter.clone()))?;

    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(counter.clone()))
            .app_data(Data::new(config.clone()))
            .app_data(Data::new(weather.clone()))
            .wrap(prometheus.clone())
            .service(image)
            .service(healthz)
            .wrap(Logger::default().exclude("/healthz").exclude("/metrics"))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(2)
    .run()
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() {
    if let Err(err) = run().await {
        log::error!("{err}");
        std::process::exit(1);
    }
}
