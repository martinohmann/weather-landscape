use actix_web::error::ResponseError;
use epd_waveshare::graphics::VarDisplayError;
use std::fmt::Display;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
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
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

impl Error {
    pub(crate) fn new(msg: impl Display) -> Self {
        Error::Message(msg.to_string())
    }
}

impl ResponseError for Error {}

impl From<VarDisplayError> for Error {
    fn from(err: VarDisplayError) -> Self {
        Error::new(format!("VarDisplay error: {err:?}"))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::new(err)
    }
}
