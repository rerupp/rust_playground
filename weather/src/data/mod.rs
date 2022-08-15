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
use std::fmt::Formatter;

use chrono::prelude::*;

use crate::core;
use crate::domain::{DailyHistories, HistoryDates, HistorySummary, Locations};

/// The module containing the filesystem implementation.
mod fs;

/// The Result returned from the data module.
///
/// Currently it's a type alias to the core module but in the future
/// it could be abstracted into a module specific implementation.
pub type DataResult<T> = core::WeatherResult<T>;

/// The type of error returned from the data module.
///
/// Currently it's a type alias to the core module but in the future
/// it could be abstracted into a module specific implementation.
pub type DataError = core::WeatherError;

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
    fn get_daily_history(&self, daily_query: DailyHistoryQuery) -> DataResult<DailyHistories>;

    /// Returns a summary of location weather data.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_history_summary(&self, history_query: HistoryQuery) -> DataResult<HistorySummary>;

    /// Returns the weather data dates for locations.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_history_dates(&self, query: HistoryQuery) -> DataResult<HistoryDates>;

    /// Provides access to the locations defined for the weather data.
    ///
    /// # Arguments
    ///
    /// * `history_query` - the search parameters used to get a summary of the weather data.
    ///
    fn get_location_data(&self, location_query: LocationQuery) -> DataResult<Locations>;
}

/// Returns a `DataAPI` filesystem based implementation.
///
/// This is used by data consumers to create an instance of the `DataAPI`.
///
/// # Arguments
///
/// * `root_pathname` - the directory name containing weather data.
///
pub fn from_pathname(root_pathname: &str) -> DataResult<Box<dyn DataAPI>> {
    Ok(fs::FsData::new_api(root_pathname)?)
}

/// The parameters controlling what locations are of interest.
pub struct LocationQuery {
    /// Identifies what locations are of interest.
    ///
    /// If filters are available they will be compared against the location name and alias.
    /// The filtering is a partial match. As an example, if you have a location named
    /// `Tigard, OR` a filter `Tig` will match however `OR` will not.
    ///
    pub filters: Vec<String>,
    /// Determines if the filter is case insensitive.
    ///
    /// As an example, if you have a location named `Las Vegas, NV` the filter `las` will match
    /// if `true` and will not if `false`.
    pub case_insensitive: bool,
    /// Determines if the returned locations should be sorted by their name.
    pub sort: bool,
}

/// The parameters controlling what location is used to return a summary of weather data.
pub struct HistoryQuery {
    /// The location id of the weather data.
    pub location_id: String,
    /// The data returned should be sorted by date.
    pub sort: bool,
}

/// The parameters controlling what location weather data should be returned.
pub struct DailyHistoryQuery {
    /// The location id of the weather data.
    pub location_id: String,
    /// The range of weather data to return.
    pub history_bounds: HistoryBounds,
}

/// The history dates used when querying weather data.
// pub struct HistoryBounds(pub Date<Utc>, pub Date<Utc>);
pub struct HistoryBounds {
    /// The lower date boundary.
    pub lower: Date<Utc>,
    /// The inclusive upper date boundary.
    pub upper: Date<Utc>,
}

impl std::fmt::Display for HistoryBounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.lower, self.upper)
    }
}

impl HistoryBounds {
    pub fn new(lower: Date<Utc>, upper: Date<Utc>) -> HistoryBounds {
        HistoryBounds { lower, upper }
    }
    /// Returns true if the date is within the lower and upper dates.
    ///
    /// # Arguments
    ///
    /// * `data` - the date that will be compared.
    pub fn contains(&self, date: &Date<Utc>) -> bool {
        &self.lower <= date && date <= &self.upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_bounds() {
        let history_bounds = HistoryBounds::new(Utc.ymd(2022, 7, 1),
                                                Utc.ymd(2022, 7, 31));
        assert_eq!(history_bounds.contains(&Utc.ymd(2022, 6, 30)), false);
        assert!(history_bounds.contains(&Utc.ymd(2022, 7, 1)));
        assert!(history_bounds.contains(&Utc.ymd(2022, 7, 31)));
        assert_eq!(history_bounds.contains(&Utc.ymd(2022, 8, 1)), false);
    }
}