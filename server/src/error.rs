use actix_web::error::ResponseError;
use std::fmt::Display;

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
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn new(msg: impl Display) -> Self {
        Error::Message(msg.to_string())
    }
}

impl ResponseError for Error {}
