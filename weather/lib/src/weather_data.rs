//! The new version of the weather data API.
use super::{backend, Result};
use crate::prelude::{
    DailyHistories, DataCriteria, DateRange, HistoryClient, HistoryDates, HistorySummaries, Location, LocationCriteria,
};
use std::path::PathBuf;
use toolslib::stopwatch::StopWatch;

/// Creates the weather data `API` depending on the backend configuration.
///
/// # Arguments
///
/// * `dirname` is the weather data directory name.
pub fn create_weather_data(config_file: Option<PathBuf>, dirname: Option<PathBuf>, no_db: bool) -> Result<WeatherData> {
    let data_api = backend::data_api(config_file, dirname, no_db)?;
    Ok(WeatherData(data_api))
}

macro_rules! log_elapsed {
    ($what:expr, $stopwatch:expr) => {
        log::info!("WeatherData: {} {}", $what, $stopwatch)
    };
}

/// The weather data `API`.
pub struct WeatherData(
    /// The weather data implementation.
    backend::DataAPI,
);
impl WeatherData {
    /// Add weather data history for a location.
    ///
    /// # Arguments
    ///
    /// - `histories` has the location and histories to add.
    ///
    pub fn add_histories(&self, daily_histories: DailyHistories) -> Result<usize> {
        self.0.add_daily_histories(daily_histories)
    }
    /// Get the client that retrieves weather history for a location.
    ///
    pub fn get_history_client(&self) -> Result<Box<dyn HistoryClient>> {
        crate::history_client::get(self.0.get_config())
    }
    /// Get daily weather history for a location.
    ///
    /// It is an error if more than 1 location is found.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the location.
    /// * `history_range` covers the history dates returned.
    ///
    pub fn get_daily_history(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories> {
        let stopwatch = StopWatch::start_new();
        let daily_history = self.0.get_daily_history(criteria, history_range)?;
        log_elapsed!("get_daily_history", &stopwatch);
        Ok(daily_history)
    }
    /// Get the history dates for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations.
    ///
    pub fn get_history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
        let stopwatch = StopWatch::start_new();
        let history_dates = self.0.get_history_dates(criteria)?;
        log_elapsed!("get_history_dates", &stopwatch);
        Ok(history_dates)
    }
    /// Get a summary of location weather data.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations.
    ///
    pub fn get_history_summary(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
        let stopwatch = StopWatch::start_new();
        let history_summary = self.0.get_history_summary(criteria)?;
        log_elapsed!("get_history_summary", &stopwatch);
        Ok(history_summary)
    }
    /// Get the weather location metadata.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations of interest.
    ///
    pub fn get_locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
        let stopwatch = StopWatch::start_new();
        let locations = self.0.get_locations(criteria)?;
        log_elapsed!("get_locations", &stopwatch);
        Ok(locations)
    }
    /// Search for locations that can be added to weather data.
    ///
    /// # Arguments
    ///
    /// - `criteria` provides the search parameters.
    ///
    pub fn search_locations(&self, criteria: LocationCriteria) -> Result<Vec<Location>> {
        let locations = self.0.search_locations(criteria)?;
        Ok(locations)
    }
    /// Add a location to weather data.
    ///
    /// # Arguments
    ///
    /// - `location` is the location that will be added.
    ///
    pub fn add_location(&self, location: Location) -> Result<()> {
        self.0.add_location(location)?;
        Ok(())
    }
}
