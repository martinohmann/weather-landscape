mod error;
mod render;

use actix_web::{
    get, http::header::ContentType, middleware, App, HttpResponse, HttpServer, Result,
};
use render::Renderer;

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[get("/image.bmp")]
async fn image_bmp() -> Result<HttpResponse> {
    let renderer = Renderer::new();
    let image = renderer.render_image()?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime::IMAGE_BMP))
        .body(render::bmp_bytes(&image)?))
}

#[get("/image.epd")]
async fn image_epd() -> Result<HttpResponse> {
    let renderer = Renderer::new();
    let image = renderer.render_image()?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(render::epd_bytes(&image)?))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
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
