#![allow(dead_code)]

use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::fmt::{self, Display, Formatter};

const BASE_URL: &str = "https://texttv.nu/api";

#[derive(Debug, Deserialize)]
pub struct Breadcrumb {
    name: String,
    url: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    num: u16,
}

#[derive(Debug, Deserialize)]
pub struct PageResponse {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    num: u16,
    title: String,
    content: Vec<String>,
    content_plain: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    next_page: u16,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    prev_page: u16,
    date_updated_unix: i64,
    permalink: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    id: u64,
    breadcrumbs: Vec<Breadcrumb>,
}

impl Display for PageResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(plain) = &self.content_plain {
            for page in plain {
                f.write_str(page)?
            }
        }
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("api error: {0}")]
    ApiStatus(u16),
    #[error("transport error: {0:?}")]
    Transport(ureq::ErrorKind),
    #[error("invalid page number: {0}")]
    InvalidPageNumber(u16),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(s, ..) => Self::ApiStatus(s),
            ureq::Error::Transport(t) => Self::Transport(t.kind()),
        }
    }
}

pub fn get_page_range(lo: u16, hi: u16) -> Result<Vec<PageResponse>, Error> {
    let url = format!("{BASE_URL}/get/{lo}-{hi}");
    let response = ureq::get(&url)
        .query("includePlainTextContent", "1")
        .call()?;

    let pages: Vec<PageResponse> = response.into_json()?;
    Ok(pages)
}

pub fn get_page(number: u16) -> Result<PageResponse, Error> {
    let url = format!("{BASE_URL}/get/{number}");
    let response = ureq::get(&url)
        .query("includePlainTextContent", "1")
        .call()?;

    let mut pages: Vec<PageResponse> = response.into_json()?;
    match pages.pop() {
        Some(page) => Ok(page),
        None => Err(Error::InvalidPageNumber(number)),
    }
}
