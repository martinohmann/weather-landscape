use actix_web::error::ResponseError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl ResponseError for Error {}
