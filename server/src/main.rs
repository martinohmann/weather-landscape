mod app;
mod config;
mod error;
mod graphics;
mod sun;
mod weather;

use crate::{
    app::{AppState, Metrics},
    config::Config,
    error::Error,
};
use actix_web::{
    get,
    http::header::ContentType,
    middleware::Logger,
    web::{Data, Path, Query},
    App, HttpResponse, HttpServer,
};
use actix_web_prom::PrometheusMetricsBuilder;
use serde::Deserialize;

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
    state: Data<AppState>,
    format: Path<ImageFormat>,
    query: Query<ImageRequest>,
) -> actix_web::Result<HttpResponse> {
    let mut data = state.weather.get().await?;

    if query.cause_havoc {
        weather::cause_havoc(&mut data);
    }

    let image = state.renderer.render(&data)?;

    let (mime_type, body) = match format.into_inner() {
        ImageFormat::Bmp => (mime::IMAGE_BMP, image.bmp_bytes()?),
        ImageFormat::Epd => (mime::APPLICATION_OCTET_STREAM, image.epd_bytes()?),
    };

    state.metrics.image_counter(mime_type.essence_str()).inc();

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime_type))
        .body(body))
}

async fn run() -> anyhow::Result<()> {
    let config = Config::load()?;

    let namespace = env!("CARGO_PKG_NAME").replace('-', "_");
    let prometheus = PrometheusMetricsBuilder::new(&namespace)
        .endpoint("/metrics")
        .build()
        .map_err(Error::new)?;

    let metrics = Metrics::new(&namespace, &prometheus.registry)?;
    let state = AppState::new(&config, metrics)?;

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
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
    tracing_subscriber::fmt::init();

    if let Err(err) = run().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}
