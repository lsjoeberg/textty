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
pub struct PageResponse {
    /// The page number.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub num: u16,

    /// The page title.
    pub title: String,

    /// The page content as HTML.
    pub content: Vec<String>,

    /// The page content as plain text; only returned when the query parameter
    /// `includePlainTextContent=1` is specified.
    pub content_plain: Option<Vec<String>>,

    /// The next available page, after `num`.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub next_page: u16,

    /// The previous available page, before `num`.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub prev_page: u16,

    /// A UNIX timestamp representing the time the page was last updated.
    pub date_updated_unix: i64,

    /// A permanent link to the current page state.
    pub permalink: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]

    /// A unique ID for the current page state.
    pub id: u64,

    /// A list of links relative to the public host of the web service,
    /// `https://texttv.nu`, defining the current page's location within
    /// the site's hierarchical structure.
    pub breadcrumbs: Vec<Breadcrumb>,
}

#[derive(Debug, Deserialize)]
pub struct Breadcrumb {
    /// The name of the page.
    pub name: String,

    /// The relative URL of the page.
    pub url: String,

    /// The page number.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub num: u16,
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
/// * Returns [`Error::InvaldPageRange`] if `lo` is not less than or equal to `hi`.
/// * Returns [`ureq::Error`] if API request fails in the network, I/O, or application stack.
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
/// * Returns [`ureq::Error`] if API request fails in the network, I/O, or application stack.
pub fn get_page(number: PageNumber) -> Result<PageResponse, Error> {
    let url = format!("{BASE_URL}/get/{}", number.0);
    let mut response = ureq::get(&url).query("app", APP_ID).call()?;

    let mut pages: Vec<PageResponse> = response.body_mut().read_json()?;
    match pages.pop() {
        Some(page) => Ok(page),
        None => Err(Error::InvalidPageNumber(number.0)),
    }
}
