//! # The weather data domain.
//!
//! I won't go into reasons why separating the front-end from the back-end is a good
//! idea but this module bridges the two layers. Here are some key points
//! of this module.
//!
//! * The domain is agnostic as to what the implementation of the back-end storage is.
//! It is composed from a `DataAPI` which has the responsibility of accessing weather data.
//! * The beans in the domain reflect the CLI use cases.
//! * The back-end is responsible for building the contents of the data beans.
//! * The front-end is uses the bean data to implement reports.
//!
pub use beans::DailyHistories as DailyHistories;
pub use beans::DailyHistory as DailyHistory;
pub use beans::DailyHistoryQuery as DailyHistoryQuery;
pub use beans::HistoryDates as HistoryDates;
pub use beans::HistoryRange as HistoryRange;
pub use beans::HistorySummary as HistorySummary;
pub use beans::Location as Location;
pub use beans::LocationQuery as LocationQuery;

use crate::core;
use crate::data;

mod beans;

/// The result of calling the domain API.
///
/// Currently it's an alias to the core result type but in the future
/// it could change.
type DomainResult<T> = core::WeatherResult<T>;

/// An error that can be returned from the domain api.
///
/// Currently it's an alias to the core error but in the future
/// it could change.
#[allow(dead_code)]
type DomainError = core::WeatherError;

/// The daily histories for a location.
///
/// This will be `None` if the location does not exist otherwise both
/// location and daily histories will be returned in the tuple.
pub type LocationDailyHistories = Option<(Location, DailyHistories)>;

/// A list of locations.
pub type Locations = Vec<Location>;

/// A list of locations and their respective history dates.
pub type LocationHistoryDates = Vec<(Location, HistoryDates)>;

/// A list of locations and the respective history summary information.
pub type LocationHistorySummaries = Vec<(Location, HistorySummary)>;

/// The public domain API.
///
/// The API is composed of a `DataAPI`. Something at a higher level is responsible for assembly
/// of the domain and data API. In this case it is the CLI `main` program that does that.
pub struct WeatherData {
    /// The data API used to gain access to the weather data.
    data_api: Box<dyn data::DataAPI>,
}

/// The domains implementation of weather data.
impl WeatherData {
    /// Create a new instance of the weather API.
    ///
    /// # Arguments
    ///
    /// * `data_api` - the data API used to access weather data.
    pub fn new(data_api: Box<dyn data::DataAPI>) -> WeatherData {
        WeatherData { data_api }
    }

    /// Returns the daily weather data history for a location.
    ///
    /// Currently it is an error if more than 1 location is found through the location
    /// query. The [get locations](WeatherData::get_locations) function is used
    /// to find the location. The DataAPI [get daily history](data::DataAPI::get_daily_history)
    /// function is used to get the historical weather data.
    ///
    /// # Arguments
    ///
    /// * `query` - the location query
    /// * `history_query` - the history data being asked for
    ///
    pub fn get_daily_history(&self,
                             query: LocationQuery,
                             history_query: DailyHistoryQuery) -> DomainResult<LocationDailyHistories>
    {
        let mut locations = self.get_locations(query)?;
        if locations.len() > 1 {
            Err(DomainError::new(&format!("Only 1 location can be used, found {}!", locations.len())))
        } else {
            match locations.pop() {
                None => Ok(None),
                Some(location) => {
                    let daily_histories = self.data_api.get_daily_history(data::DailyHistoryQuery {
                        location_id: location.id.clone(),
                        history_bounds: data::HistoryBounds::new(history_query.history_range.from,
                                                                 history_query.history_range.to),
                    })?;
                    let location_daily_histories = (location, daily_histories);
                    Ok(Some(location_daily_histories))
                }
            }
        }
    }

    /// Returns the history dates for locations.
    ///
    /// The [get locations](WeatherData::get_locations) function is used
    /// to find the locations and the DataAPI [get history dates](data::DataAPI::get_history_dates)
    /// function retrieves the history dates.
    ///
    /// # Arguments
    ///
    /// * `query` - the location query
    ///
    pub fn get_history_dates(&self, query: LocationQuery) -> DomainResult<LocationHistoryDates> {
        let locations = self.get_locations(query)?;
        let mut history_details: LocationHistoryDates = vec![];
        for location in locations {
            let history_dates = self.data_api.get_history_dates(data::HistoryQuery {
                location_id: location.id.clone(),
                sort: false,
            })?;
            history_details.push((location, history_dates))
        }
        Ok(history_details)
    }

    /// Returns a summary of the weather data for locations.
    ///
    /// The [get locations](WeatherData::get_locations) function is used
    /// to find the locations. The DataAPI [get history summary](data::DataAPI::get_history_summary)
    /// function retrieves the history information for each of the locations found.
    ///
    /// # Arguments
    ///
    /// * `query` - the location query
    ///
    pub fn get_history_summary(&self, query: LocationQuery) -> DomainResult<LocationHistorySummaries> {
        let locations = self.get_locations(query)?;
        let mut history_summaries: LocationHistorySummaries = vec![];
        for location_data in locations {
            let history_summary = self.data_api.get_history_summary(data::HistoryQuery {
                location_id: location_data.id.clone(),
                sort: false,
            })?;
            history_summaries.push((location_data, history_summary));
        }
        Ok(history_summaries)
    }

    /// Returns location properties.
    ///
    /// The DataAPI [get locations](data::DataAPI::get_location_data) function is used
    /// to find the locations.
    ///
    /// # Arguments
    ///
    /// * `query` - the location query
    ///
    pub fn get_locations(&self, query: LocationQuery) -> DomainResult<Locations> {
        let locations = self.data_api.get_location_data(data::LocationQuery {
            filters: query.location_filter,
            sort: query.sort,
            case_insensitive: query.case_insensitive,
        })?;
        Ok(locations)
    }
}

/// Log an error.
///
/// Currently the error is written to `stderr` however `log4rs` is in this
/// silly things future.
///
/// # Arguments
///
/// * `error` - the error that will be logged.
///
pub fn log_error(error: &str) {
    eprintln!("Domain error: {}...", error);
}
