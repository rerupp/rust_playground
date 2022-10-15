//! # The weather data back-end storage.
//!
//! This module defines the API used to access weather data. It is designed to
//! allow different implementations of the back-end storage.
//!
//! Currently there is only 1 implementation that is built on top of a filesystem. It
//! uses `JSON` and `ZIP archives` as the storage facility. I suspect before my fun
//! with RUST is done I will have some type of database implementation built.
//!
//! Arguably the data module could live as a module within the domain module
//! but the peer relationship seems better versus a child relationship.
//!
use std::{
    fmt,
    io,
    result,
};

use chrono::prelude::*;

// use crate::core;
// use super::{domain, Error, Result};
use super::domain;
// use super::domain::{DailyHistories, HistoryDates, HistorySummary, Locations};

/// The module containing the filesystem implementation.
// mod fs;

mod objects;
mod files;

pub use objects::DailyHistoryQuery;
pub use objects::HistoryQuery;
pub use objects::LocationQuery;
pub use objects::HistoryBounds;

/// The Result returned from the data module.
pub type Result<T> = result::Result<T, Error>;

/// The type of error returned from the data module.
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&Error> for String {
    fn from(error: &Error) -> Self {
        error.0.clone()
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(error.as_str())
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(format!("data: {error}"))
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error(format!("io: {error}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error(format!("serde_json: {error}"))
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(error: zip::result::ZipError) -> Self {
        Error(format!("zip: {error}"))
    }
}

/// The data domain API.
///
pub trait DataAPI {
    /// Returns historical daily weather data for a location.
    ///
    /// # Arguments
    ///
    /// * `daily_query` - the search parameters used to find historical weather data for a
    /// location.
    ///
    fn get_daily_history(&self, daily_query: DailyHistoryQuery) -> Result<domain::DailyHistories>;

    /// Returns a summary of location weather data.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_history_summary(&self, history_query: HistoryQuery) -> Result<domain::HistorySummary>;

    /// Returns the weather data dates for locations.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_history_dates(&self, query: HistoryQuery) -> Result<domain::HistoryDates>;

    /// Provides access to the locations defined for the weather data.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_location_data(&self, location_query: LocationQuery) -> Result<domain::Locations>;
}

/// Returns a `DataAPI` filesystem based implementation.
///
/// This is used by data consumers to create an instance of the `DataAPI`.
///
/// # Arguments
///
/// * `root_pathname` - the directory name containing weather data.
///
pub(crate) fn get_filesapi(root_pathname: &str) -> Result<Box<dyn DataAPI>> {
    Ok(files::FsData::new_api(root_pathname)?)
}
