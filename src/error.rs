#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("transport error: {0:?}")]
    Transport(#[from] ureq::Error),
    #[error("invalid page number: {0}")]
    InvalidPageNumber(u16),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("error parsing HTML: {0}")]
    ParseHtml(String),
}
