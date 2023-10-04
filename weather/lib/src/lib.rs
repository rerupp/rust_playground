//! A library containing the weather data backend API and implementation.
//!
//! This implementation is loosely base on a `Python` project I created several years ago. When
//! I started the `Python` project I wanted to chart historical weather information for different
//! areas we were interested in spending the winter. The idea of building a CLI based on the
//! original weather data implementation seemed like a fun journey.
//!
//! # History
//!
//! The `Python` weather data project is based on *DarkSky* weather history data.
//! Unfortunately the API was purchased by Apple and is no longer publicly available (or at least
//! free) but I had collected years of data for a dozen or more sites. Initially the `Rust` implementation
//! faithfully followed the `Python` implementation using the *DarkSky* data.
//!
//! # October 2023 Version
//! 
//! Late summer I came across *Visual Crossings* and their *Timeline* weather history API. It had
//! most of the historical weather data I was interested in so I decided to support adding weather
//! history using their API. The biggest change behind this move was storing weather history in a new
//! `JSON` document format. Both *DarkSky* and *Timeline* historical data are supersets of the data
//! currently being stored. I decided this was the best approach in case *Timeline* goes away and a new
//! weather history API needs to be used. 

// Ignore broke links due to --document-private-items not being used.
#![allow(rustdoc::private_intra_doc_links)]

/// The weather data implementation is scoped to the library.
pub(crate) mod backend;

/// The public data structures.
pub mod prelude {
    pub use crate::{
        api::WeatherData,
        entities::{
            DailyHistories, DailyHistory, DataCriteria, DateRange, DateRanges, DbConfig, History, HistoryDates,
            HistorySummaries, HistorySummary, Location,
        },
    };
}

/// The public administation data structures.
pub mod admin_prelude {
    pub use crate::{create_weather_admin, entities::DbConfig, WeatherAdmin};
}
use std::{fmt::Display, result};

/// The library result.
pub type Result<T> = result::Result<T, Error>;

/// The library error.
#[derive(Debug)]
pub struct Error(String);
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<backend::Error> for Error {
    /// Support converting from the implemenation error.
    fn from(error: backend::Error) -> Self {
        Error(error.to_string())
    }
}
impl From<String> for Error {
    /// Create an error from the provided string.
    fn from(error: String) -> Self {
        Error(error)
    }
}
impl From<&str> for Error {
    /// Create an error from the provided string.
    fn from(error: &str) -> Self {
        Error(error.to_string())
    }
}

pub use api::{archive_weather_data, db_weather_data};
mod api {
    //! The new version of the weather data API.

    use super::{backend, Error, Result};
    use crate::prelude::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location};
    use toolslib::stopwatch::StopWatch;

    /// Creates the weather data `API` for zip archives.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather data directory name.
    pub fn archive_weather_data(dirname: &str) -> Result<WeatherData> {
        let adapter = backend::filesys::archive_adapter(dirname)?;
        Ok(WeatherData(adapter))
    }

    /// Creates the weather data `API` for databases.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather data directory name.
    pub fn db_weather_data(dirname: &str) -> Result<WeatherData> {
        let adapter = backend::db::data_adapter(dirname)?;
        Ok(WeatherData(adapter))
    }

    macro_rules! log_elapsed {
        ($what:expr, $stopwatch:expr) => {
            log::info!("WeatherData: {} {}", $what, $stopwatch)
        };
    }
    /// The weather data `API`.
    pub struct WeatherData(
        /// The backend implementation of weather data.
        Box<dyn backend::DataAdapter>,
    );
    impl WeatherData {
        /// Add weather history to a location.
        ///
        /// It is an error if more than 1 location is found.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the location.
        /// * `date_range` covers the history dates returned.
        pub fn add_history(&self, criteria: DataCriteria, date_range: DateRange) -> Result<usize> {
            let stopwatch = StopWatch::start_new();
            let location = self.get_location(&criteria)?;
            let histories = backend::get_weather_history(&location, &date_range)?;
            let count = self.0.add_histories(&DailyHistories { location, histories })?;
            log_elapsed!("add_history", &stopwatch);
            Ok(count)
        }
        /// Get daily weather history for a location.
        ///
        /// It is an error if more than 1 location is found.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the location.
        /// * `history_range` covers the history dates returned.
        pub fn get_daily_history(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories> {
            let stopwatch = StopWatch::start_new();
            let location = self.get_location(&criteria)?;
            let data = self.0.daily_histories(location, history_range)?;
            log_elapsed!("get_daily_history", &stopwatch);
            Ok(data)
        }
        /// Get the history dates for locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations.
        pub fn get_history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
            let stopwatch = StopWatch::start_new();
            let data = self.0.history_dates(criteria)?;
            log_elapsed!("get_history_dates", &stopwatch);
            Ok(data)
        }
        /// Get a summary of location weather data.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations.
        pub fn get_history_summary(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
            let stopwatch = StopWatch::start_new();
            let data = self.0.history_summaries(DataCriteria::from(criteria))?;
            log_elapsed!("get_history_summary", &stopwatch);
            Ok(data)
        }
        /// Get the weather location metadata.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations of interest.
        pub fn get_locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
            let stopwatch = StopWatch::start_new();
            let data = self.0.locations(criteria)?;
            log_elapsed!("get_locations", &stopwatch);
            Ok(data)
        }
        /// Used internally to get a single location, error otherwise.
        /// 
        /// # Arguments
        /// 
        /// * `criteria` is the location being searched for.
        fn get_location(&self, criteria: &DataCriteria) -> Result<Location> {
            let mut locations = self.get_locations(DataCriteria {
                filters: criteria.filters.clone(),
                icase: criteria.icase,
                sort: criteria.sort,
            })?;
            match locations.len() {
                1 => Ok(locations.pop().unwrap()),
                0 => Err(Error::from("A location was not found.")),
                _ => Err(Error::from("Multiple locations were found.")),
            }
        }
    }
}

pub use admin_api::{create as create_weather_admin, WeatherAdmin};
mod admin_api {
    //! The weather data adminstation API.
    use super::{
        backend::{
            db::admin,
            filesys::{migrate_history, weather_dir, MigrateConfig, WeatherDir},
        },
        entities::{DataCriteria, DbConfig, DbInfo},
        Result,
    };
    use std::path::PathBuf;

    /// Create an instance of the weather data administration `API`.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather data directory pathname.
    pub fn create(dirname: &str) -> crate::Result<WeatherAdmin> {
        Ok(WeatherAdmin(weather_dir(dirname)?))
    }

    /// The weather data adminstration `API`.
    #[derive(Debug)]
    pub struct WeatherAdmin(
        /// The weather data directory.
        WeatherDir,
    );
    impl WeatherAdmin {
        /// Initialize the weather database using the supplied databse configuration.
        ///
        /// # Arguments
        ///
        /// * `db_config` is the database configuration.
        /// * `drop` when `true` will delete the schema before initialization.
        /// * `load` when `true` will load weather data into the database.
        pub fn init(&self, db_config: DbConfig, drop: bool, load: bool, threads: usize) -> Result<()> {
            admin::init_db(&self.0, db_config, drop, load, threads)?;
            Ok(())
        }
        /// Deletes the weather database schema and optionally deletes the database.
        ///
        /// # Arguments
        ///
        /// * `delete` when `true` will delete the database file.
        pub fn drop(&self, delete: bool) -> Result<()> {
            admin::drop_db(&self.0, delete)?;
            Ok(())
        }
        /// Provides information about the weather data database.
        pub fn stat(&self) -> crate::Result<DbInfo> {
            let db_config = admin::stat(&self.0)?;
            Ok(db_config)
        }
        /// Convert *DarkSky* archives into [History](crate::entities::History) archives.
        /// 
        /// # Arguments
        /// 
        /// * `into` identifies the directory where converted archive will be written.
        /// * `create` indicates the directory should be created if it does not exist.
        /// * `retain` indicates existing converted archives should not be deleted before adding documents.
        /// * `criteria` identifies what location archives should be converted.
        pub fn migrate(&self, into: PathBuf, create: bool, retain: bool, criteria: DataCriteria) -> Result<usize> {
            let count = migrate_history(MigrateConfig { source: &self.0, create, retain, criteria }, into)?;
            Ok(count)
        }
    }
}

mod entities {
    //! Structures used by the weather data `API`s.

    use chrono::{NaiveDate, NaiveDateTime};

    /// Used by front-ends to identify locations.
    #[derive(Debug)]
    pub struct DataCriteria {
        /// The locations of iterest.
        pub filters: Vec<String>,
        /// If `true` the location filters will ignore case.
        pub icase: bool,
        /// If `true` locations will be sorted by name.
        pub sort: bool,
    }

    /// A locations daily weather history.
    #[derive(Debug)]
    pub struct DailyHistories {
        /// The location metadata.
        pub location: Location,
        /// The daily histories for a location.
        pub histories: Vec<History>,
    }

    /// A locations history dates.
    #[derive(Debug)]
    pub struct HistoryDates {
        /// The location metadata.
        pub location: Location,
        /// The history dates metadata.
        pub history_dates: Vec<DateRange>,
    }

    #[derive(Debug)]
    /// A locations history summary.
    pub struct HistorySummaries {
        pub location: Location,
        /// The number of weather data histories available.
        pub count: usize,
        /// The overall size of weather data in bytes (may or may not be available).
        pub overall_size: Option<usize>,
        /// The size in bytes of weather data.
        pub raw_size: Option<usize>,
        /// The size in bytes of weather data in the backing store.
        pub store_size: Option<usize>,
    }

    /// The data that comprises a location.
    #[derive(Debug)]
    pub struct Location {
        /// The name of a location.
        pub name: String,
        /// A unique nickname of a location.
        pub alias: String,
        /// The location longitude.
        pub longitude: String,
        /// The location latitude.
        pub latitude: String,
        /// the location timezone.
        pub tz: String,
    }

    /// A locations history summary.
    #[derive(Debug)]
    pub struct HistorySummary {
        /// The location id.
        pub location_id: String,
        /// The number of weather data histories available.
        pub count: usize,
        /// The overall size of weather data for a location in bytes (may or may not be available).
        pub overall_size: Option<usize>,
        /// The raw size of weather data for a location in bytes (may or may not be available).
        pub raw_size: Option<usize>,
        /// The compressed data size of weather data for a location in bytes (may or may not be available).
        pub compressed_size: Option<usize>,
    }

    /// The weather history data.
    #[derive(Debug)]
    pub struct History {
        /// The location alias name.
        pub alias: String,
        /// The history date.
        pub date: NaiveDate,
        /// The high temperature for the day.
        pub temperature_high: Option<f64>,
        /// The low temperature for the day.
        pub temperature_low: Option<f64>,
        /// The daily mean temperature.
        pub temperature_mean: Option<f64>,
        /// The dew point temperature.
        pub dew_point: Option<f64>,
        /// The relative humidity percentage.
        pub humidity: Option<f64>,
        /// The chance of rain during the day.
        pub precipitation_chance: Option<f64>,
        /// A short description of the type of rain.
        pub precipitation_type: Option<String>,
        /// The amount of percipitation for the day.
        pub precipitation_amount: Option<f64>,
        /// The daily wind speed.
        pub wind_speed: Option<f64>,
        /// The highest wind speed recorded for the day.
        pub wind_gust: Option<f64>,
        /// The general direction in degrees.
        pub wind_direction: Option<i64>,
        /// The percentage of sky covered by clouds.
        pub cloud_cover: Option<f64>,
        /// The daily atmospheric pressus expressed in millibars.
        pub pressure: Option<f64>,
        /// The level of ultra violet exposure for the day.
        pub uv_index: Option<f64>,
        /// The local time when the sun comes up.
        pub sunrise: Option<NaiveDateTime>,
        /// The local time when the sun will set.
        pub sunset: Option<NaiveDateTime>,
        /// The moons phase between 0 and 1.
        pub moon_phase: Option<f64>,
        /// The distance that can be during the day.
        pub visibility: Option<f64>,
        /// A summary of the daily weather.
        pub description: Option<String>,
    }

    /// The daily weather data.
    #[derive(Debug)]
    pub struct DailyHistory {
        /// The location id.
        pub location_id: String,
        /// The date associated with the weather data.
        pub date: NaiveDate,
        /// The high temperature.
        pub temperature_high: Option<f64>,
        /// The high temperature time of day.
        pub temperature_high_time: Option<i64>,
        /// The low temperature.
        pub temperature_low: Option<f64>,
        /// The low temperature time of day.
        pub temperature_low_time: Option<i64>,
        /// The maximum temperature.
        pub temperature_max: Option<f64>,
        /// The maximum temperature time of day.
        pub temperature_max_time: Option<i64>,
        /// The minimum temperature.
        pub temperature_min: Option<f64>,
        /// The minimum temperature time of day.
        pub temperature_min_time: Option<i64>,
        /// The average wind speed.
        pub wind_speed: Option<f64>,
        /// The maximum wind speed.
        pub wind_gust: Option<f64>,
        /// The maximum wind speed time of day.
        pub wind_gust_time: Option<i64>,
        /// The predominate wind direction.
        pub wind_bearing: Option<i64>,
        /// The percent of cloud cover.
        pub cloud_cover: Option<f64>,
        /// The UV index.
        pub uv_index: Option<i64>,
        /// The UV time of day.
        pub uv_index_time: Option<i64>,
        /// A textual summary of the daily weather.
        pub summary: Option<String>,
        /// The average humidity.
        pub humidity: Option<f64>,
        /// The dew point.
        pub dew_point: Option<f64>,
        /// The sunrise time of day.
        pub sunrise_time: Option<i64>,
        /// The sunset time of day.
        pub sunset_time: Option<i64>,
        /// The phase of the moon.
        pub moon_phase: Option<f64>,
    }

    /// For a given `NaiveDate` return the next day `NaiveDate`.
    macro_rules! next_day {
        ($nd:expr) => {
            // For the weather data use case this should always be okay
            $nd.succ_opt().unwrap()
        };
    }

    /// A locations weather data history dates.
    #[derive(Debug)]
    pub struct DateRanges {
        /// The location id.
        pub location_id: String,
        /// The location weather history dates, grouped as consecutive date ranges.
        pub date_ranges: Vec<DateRange>,
    }
    impl DateRanges {
        pub fn covers(&self, date: &NaiveDate) -> bool {
            self.date_ranges.iter().any(|date_range| date_range.covers(date))
        }
    }

    /// A container for a range of dates.
    #[derive(Debug)]
    pub struct DateRange {
        /// The starting date of the range.
        pub from: NaiveDate,
        /// The inclusive end date of the range.
        pub to: NaiveDate,
    }
    impl DateRange {
        /// Create a new instance of the date range.
        ///
        /// # Arguments
        ///
        /// * `from` is the starting date.
        /// * `thru` is the inclusize end date.
        pub fn new(from: NaiveDate, thru: NaiveDate) -> DateRange {
            DateRange { from, to: thru }
        }
        /// Returns `true` if the *from* and *to* dates are equal.
        pub fn is_one_day(&self) -> bool {
            &self.from == &self.to
        }
        /// Identifies if a date is within the date range.
        ///
        /// # Arguments
        ///
        /// * `date` is the date that will be checked.
        pub fn covers(&self, date: &NaiveDate) -> bool {
            date >= &self.from && date <= &self.to
        }
        /// Allow the history range to be iterated over without consumming it.
        pub fn as_iter(&self) -> DateRangeIterator {
            DateRangeIterator { from: self.from.clone(), thru: self.to.clone() }
        }
        /// Returns the dates as a tuple of ISO8601 formatted strings.
        pub fn as_iso8601(&self) -> (String, String) {
            use toolslib::date_time::isodate;
            (isodate(&self.from), isodate(&self.to))
        }
        /// A helper that builds a list of history range from a list of dates.
        ///
        /// As an example, if the following date list was passed to the function:
        ///
        /// * 2022-08-12
        /// * 2022-08-10
        /// * 2022-08-14
        ///
        /// The resulting list of date ranges would be returned.
        ///
        /// * (2022-08-10, 2022-08-10)
        /// * (2022-08-12, 2022-08-14)
        ///
        /// # Arguments
        ///
        /// * `dates` - The list of dates that will be converted to date ranges.
        ///
        pub fn from_dates(mut dates: Vec<NaiveDate>) -> Vec<DateRange> {
            // let mut dates = dates.clone();
            // dates.sort_by(|lhs, rhs| lhs.cmp(rhs));
            dates.sort_unstable();
            let mut history_ranges = vec![];
            let dates_len = dates.len();
            if dates_len == 1 {
                history_ranges.push(DateRange::new(dates[0], dates[0]));
            } else if dates_len > 1 {
                let mut from = dates[0];
                let mut to = dates[0];
                for i in 1..dates_len {
                    if next_day!(to) != dates[i] {
                        history_ranges.push(DateRange::new(from, to));
                        from = dates[i];
                        to = dates[i];
                    } else {
                        to = dates[i];
                    }
                }
                history_ranges.push(DateRange::new(from, to));
            }
            history_ranges
        }
    }
    /// Create an iterator that will return all dates within the range.
    impl IntoIterator for DateRange {
        type Item = NaiveDate;
        type IntoIter = DateRangeIterator;
        fn into_iter(self) -> Self::IntoIter {
            DateRangeIterator { from: self.from, thru: self.to }
        }
    }

    #[derive(Debug)]
    /// Create the DateRange iterator structure.
    ///
    /// # Arguments
    ///
    /// * `from` is the starting date.
    /// * `thru` is the inclusive end date.
    pub struct DateRangeIterator {
        /// The starting date.
        from: NaiveDate,
        /// The inclusize end date.
        thru: NaiveDate,
    }
    /// The implementation of iterating over the date range.
    impl Iterator for DateRangeIterator {
        type Item = NaiveDate;
        fn next(&mut self) -> Option<Self::Item> {
            if self.from > self.thru {
                None
            } else {
                let date = self.from;
                self.from = next_day!(date);
                Some(date)
            }
        }
    }

    /// The database configuration settings.
    #[derive(Debug)]
    pub struct DbConfig {
        /// Configures the database to use weather data archives as the history data source.
        pub hybrid: bool,
        /// Configures the database to use `JSON` document as the history data source.
        pub document: bool,
        /// Configures the database to use a full relational model as the history data source.
        pub normalize: bool,
        /// When using documents, controls if the `JSON` documents will be compressed or not.
        pub compress: bool,
    }
    impl DbConfig {
        /// A helper that creates the database configuration when running in the hybrid mode.
        pub fn hybrid() -> Self {
            Self { hybrid: true, document: false, normalize: false, compress: false }
        }
        /// A helper that creates the database configuration when running in the document mode.
        pub fn document(compress: bool) -> Self {
            Self { hybrid: false, document: true, normalize: false, compress }
        }
        /// A helper that creates the database configuration when running in a fully relational mode.
        pub fn normalize() -> Self {
            Self { hybrid: false, document: false, normalize: true, compress: false }
        }
    }

    /// The database information.
    #[derive(Debug)]
    pub struct DbInfo {
        /// The database configuration.
        pub config: Option<DbConfig>,
        /// The size of the database.
        pub size: usize,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use toolslib::date_time::get_date;

        #[test]
        pub fn iterate() {
            let range = DateRange::new(get_date(2022, 6, 1), get_date(2022, 6, 30));
            let mut testcase = range.from.clone();
            let test_cases: Vec<NaiveDate> = range.into_iter().collect();
            assert_eq!(test_cases.len(), 30);
            for day in 0..30 {
                assert_eq!(test_cases[day], testcase);
                // test_case = test_case.succ();
                testcase = next_day!(testcase);
            }
        }

        #[test]
        pub fn empty_history_range() {
            assert!(DateRange::from_dates(vec![]).is_empty());
        }

        #[test]
        pub fn single_history_range() {
            let test_date = get_date(2022, 7, 6);
            let testcase = DateRange::from_dates(vec![test_date]);
            assert_eq!(testcase.len(), 1);
            assert!(testcase[0].is_one_day());
            assert_eq!(test_date, testcase[0].from);
            assert_eq!(test_date, testcase[0].to);
            let (from, to) = testcase[0].as_iso8601();
            assert_eq!(from, to);
        }

        #[test]
        fn is_within() {
            let testcase = DateRange::new(get_date(2023, 7, 1), get_date(2023, 7, 31));
            assert!(testcase.covers(&get_date(2023, 7, 1)));
            assert!(!testcase.covers(&get_date(2023, 6, 30)));
            assert!(testcase.covers(&get_date(2023, 7, 31)));
            assert!(!testcase.covers(&get_date(2023, 8, 1)));
        }

        #[test]
        pub fn multiple_history_range() {
            let test_dates =
                vec![get_date(2022, 7, 3), get_date(2022, 6, 30), get_date(2022, 7, 4), get_date(2022, 7, 1)];
            let test_case = DateRange::from_dates(test_dates.clone());
            assert_eq!(test_case.len(), 2);
            assert_eq!(test_dates[1], test_case[0].from);
            assert_eq!(test_dates[3], test_case[0].to);
            assert_eq!(test_dates[0], test_case[1].from);
            assert_eq!(test_dates[2], test_case[1].to);
        }

        #[test]
        pub fn to_iso8601_history_range() {
            let test_case = DateRange::new(get_date(2022, 7, 1), get_date(2022, 7, 2));
            let (from, to) = test_case.as_iso8601();
            assert_eq!(from, "2022-07-01");
            assert_eq!(to, "2022-07-02");
        }
    }
}
