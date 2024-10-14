use actix_web::error::ResponseError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Weather error: {0}")]
    Monsoon(#[from] monsoon::Error),
    #[error("Jiff error: {0}")]
    Jiff(#[from] jiff::Error),
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl ResponseError for Error {}
