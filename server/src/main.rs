use actix_web::{get, http::header::ContentType, middleware, App, HttpResponse, HttpServer};

// Use some random test images for now.
const PORTRAIT_IMAGE: &[u8] = include_bytes!("../../data/test_portait.bmp");
const LANDSCAPE_IMAGE: &[u8] = include_bytes!("../../data/test_landscape.bmp");

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[get("/image.bmp")]
async fn image() -> HttpResponse {
    let body = if rand::random() {
        PORTRAIT_IMAGE
    } else {
        LANDSCAPE_IMAGE
    };

    HttpResponse::Ok()
        .insert_header(ContentType(mime::IMAGE_BMP))
        .body(body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
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
