#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("transport error: {0:?}")]
    Transport(#[from] ureq::Error),
    #[error("invalid page number: {0}")]
    InvalidPageNumber(u16),
    #[error(
        "invalid page range: expected lower page {lo} to be less than or equal to upper page {hi}"
    )]
    InvalidPageRange { lo: u16, hi: u16 },
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("error parsing HTML: {0}")]
    ParseHtml(String),
}
