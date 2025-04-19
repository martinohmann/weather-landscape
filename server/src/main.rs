mod app;
mod config;
mod error;
mod graphics;
mod preset;
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
    middleware::Logger,
    web::{Data, Path, Query},
};
use actix_web_prom::PrometheusMetricsBuilder;
use jiff::Zoned;
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Deserialize, Clone, Debug)]
struct ImageQuery {
    /// Adds a lot of randomness to the weather data to make the weather seem unpredictable.
    wreck_havoc: Option<bool>,
    /// A seed for the RNG to produce stable randomness.
    seed: Option<u64>,
}

impl ImageQuery {
    fn seed_rng(&self) -> StdRng {
        let seed = self.seed.unwrap_or_else(rand::random);
        debug!(?seed, "seeding RNG used for image rendering");
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
    let settings = state.presets.get_settings_for(Zoned::now().datetime());

    let wreck_havoc = query.wreck_havoc.or(settings.wreck_havoc).unwrap_or(false);

    let mut data = state.weather.get().await?;
    let mut rng = query.seed_rng();

    if wreck_havoc {
        weather::wreck_havoc(&mut data, &mut rng);
    }

    let image = state.renderer.render(&data, rng);
    let (body, mime_type) = image.encode(format.into_inner())?;

    state.metrics.image_counter(mime_type.essence_str()).inc();

    let mut resp = HttpResponse::Ok();

    settings.configure_response(&mut resp);

    Ok(resp.content_type(mime_type).body(body))
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
        error!("{err}");
        std::process::exit(1);
    }
}
