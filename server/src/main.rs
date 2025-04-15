mod app;
mod config;
mod error;
mod graphics;
mod sun;
mod weather;

use crate::{
    app::{AppState, Metrics},
    config::Config,
    error::Result,
    graphics::ImageFormat,
};
use actix_web::{
    App, HttpResponse, HttpServer, get,
    http::header::ContentType,
    middleware::Logger,
    web::{Data, Path, Query},
};
use actix_web_prom::PrometheusMetricsBuilder;
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
struct ImageQuery {
    /// Adds a lot of randomness to the weather data to make the weather seem unpredictable.
    #[serde(default)]
    wreck_havoc: bool,
    /// A seed for the RNG to produce stable randomness.
    seed: Option<u64>,
}

impl ImageQuery {
    fn seed_rng(&self) -> StdRng {
        let seed = self.seed.unwrap_or_else(rand::random);
        tracing::debug!(?seed, "seeding RNG used for image rendering");
        StdRng::seed_from_u64(seed)
    }
}

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[get("/image.{format}")]
async fn image(
    state: Data<AppState>,
    format: Path<ImageFormat>,
    query: Query<ImageQuery>,
) -> actix_web::Result<HttpResponse> {
    let mut data = state.weather.get().await?;
    let mut rng = query.seed_rng();

    if query.wreck_havoc {
        weather::wreck_havoc(&mut data, &mut rng);
    }

    let image = state.renderer.render(&data, rng);
    let (body, mime_type) = image.encode(format.into_inner())?;

    state.metrics.image_counter(mime_type.essence_str()).inc();

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime_type))
        .body(body))
}

async fn run() -> Result<()> {
    let config = Config::load()?;

    let namespace = env!("CARGO_PKG_NAME").replace('-', "_");
    let prometheus = PrometheusMetricsBuilder::new(&namespace)
        .endpoint("/metrics")
        .build()?;

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
