#![allow(dead_code)]

use crate::error::Error;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::fmt::{self, Display, Formatter};

const BASE_URL: &str = "https://texttv.nu/api";
const APP_ID: &str = "textty";

pub const HOME_PAGE_NR: u16 = 100;
pub const MIN_PAGE_NR: u16 = 100;
pub const MAX_PAGE_NR: u16 = 801;

#[derive(Debug, Deserialize)]
pub struct Breadcrumb {
    pub name: String,
    pub url: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub num: u16,
}

#[derive(Debug, Deserialize)]
pub struct PageResponse {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub num: u16,
    pub title: String,
    pub content: Vec<String>,
    pub content_plain: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub next_page: u16,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub prev_page: u16,
    pub date_updated_unix: i64,
    pub permalink: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: u64,
    pub breadcrumbs: Vec<Breadcrumb>,
}

impl Display for PageResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(plain) = &self.content_plain {
            for page in plain {
                f.write_str(page)?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct PageNumber(u16);

impl TryFrom<u16> for PageNumber {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if (MIN_PAGE_NR..=MAX_PAGE_NR).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::InvalidPageNumber(value))
        }
    }
}

/// Get a range of pages from `texttv.nu`; from `lo` to `hi`.
///
/// # Errors
///
/// Will return `Err` if the API call fails or if the response body
/// cannot be deserialized.
pub fn get_page_range(lo: PageNumber, hi: PageNumber) -> Result<Vec<PageResponse>, Error> {
    let url = format!("{BASE_URL}/get/{}-{}", lo.0, hi.0);
    let mut response = ureq::get(&url).query("app", APP_ID).call()?;

    let pages: Vec<PageResponse> = response.body_mut().read_json()?;
    Ok(pages)
}

/// Get a single page from `texttv.nu`. If the API returns multiple
/// pages, the first on in order is returned.
///
/// # Errors
///
/// Will return `Err` if the API call fails or if the response body
/// cannot be deserialized.
pub fn get_page(number: PageNumber) -> Result<PageResponse, Error> {
    let url = format!("{BASE_URL}/get/{}", number.0);
    let mut response = ureq::get(&url).query("app", APP_ID).call()?;

    let mut pages: Vec<PageResponse> = response.body_mut().read_json()?;
    match pages.pop() {
        Some(page) => Ok(page),
        None => Err(Error::InvalidPageNumber(number.0)),
    }
}
