//! The filesystem objects that support implementing weather data using `ZIP` archives.
use super::*;

pub(crate) mod admin;

mod files;
pub(crate) use files::{WeatherDir, WeatherFile};

pub(crate) mod archives;
pub(super) use archives::{archive_history_collector, ArchiveMd, WeatherArchive, WeatherHistory, WeatherHistoryUpdate};

mod history;

mod locations;
pub(super) use locations::weather_locations;

/// Get a [WeatherDir] instance.
pub(crate) fn weather_dir(dirname: &str) -> Result<WeatherDir> {
    let weather_dir = if dirname.len() > 0 {
        WeatherDir::try_from(dirname)?
    } else if let Ok(env_pathname) = std::env::var("WEATHER_DATA") {
        WeatherDir::try_from(env_pathname)?
    } else {
        WeatherDir::try_from("weather_data")?
    };
    Ok(weather_dir)
}

pub(in crate::backend) use v1::create as data_adapter;
mod v1 {
    //! The first generation of the new file based weather data implementation
    use super::*;

    use crate::prelude::{
        DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location, LocationCriteria,
    };

    use locations::search_locations;
    use toolslib::stopwatch::StopWatch;

    /// Creates the file based data API for weather data.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather data directory name. If the directory name is
    /// empty the `WEATHER_DATA` environment variable will be used if it has been
    /// defined. Otherwise it will use the `weather_data` directory in the current
    /// working directory.
    // pub fn create(config: Config) -> Result<Box<dyn DataAdapter>> {
    pub fn create(config: Config) -> Result<Box<dyn DataAdapter>> {
        log::debug!("DataAdapter");
        let weather_dir = weather_dir(&config.weather_data.directory)?;
        Ok(Box::new(ArchiveDataAdapter { config, weather_dir }))
    }

    /// Consolidate logging elapsed time here.
    macro_rules! log_elapsed {
        (trace, $what:expr, $stopwatch:expr) => {
            log::trace!("ArchiveDataAdapter: {} {}", $what, $stopwatch)
        };
        ($what:expr, $stopwatch:expr) => {
            log::debug!("ArchiveDataAdapter: {} {}", $what, $stopwatch)
        };
    }

    /// The archive implementation of a [DataAdapter].
    struct ArchiveDataAdapter {
        config: Config,
        /// The directory containing weather data files.rs
        weather_dir: WeatherDir,
    }
    impl ArchiveDataAdapter {
        /// Used internally to get the archive manager for some location.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        fn get_archive(&self, alias: &str) -> Result<WeatherHistory> {
            let mut stopwatch = StopWatch::start_new();
            let weather_file = self.weather_dir.archive(alias);
            log_elapsed!(trace, format!("get_archive '{}' WeatherFile", alias), &stopwatch);
            stopwatch.start();
            let weather_history = WeatherHistory::new(alias, weather_file)?;
            log_elapsed!(trace, format!("get_archive '{}' WeatherHistory", alias), &stopwatch);
            Ok(weather_history)
        }
    }
    impl DataAdapter for ArchiveDataAdapter {
        /// Get the data adapter configuration.
        fn config(&self) -> &Config {
            &self.config
        }
        /// Add weather data history for a location.
        ///
        /// # Arguments
        ///
        /// * `daily_histories` has the location and histories to add.
        fn add_daily_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
            let location = &daily_histories.location;
            let file = self.weather_dir.archive(&location.alias);
            let mut archive_updater = WeatherHistoryUpdate::new(&location.alias, file)?;
            let additions = archive_updater.add(&daily_histories.histories)?;
            Ok(additions.len())
        }
        /// Returns the daily weather data history for a location.
        ///
        /// # Arguments
        ///
        /// * `location` identifies what location should be used.
        /// * `history_range` specifies the date range that should be used.
        fn daily_histories(&self, location: Location, history_range: DateRange) -> Result<DailyHistories> {
            let stopwatch = StopWatch::start_new();
            let archive = self.get_archive(&location.alias)?;
            let daily_histories = archive.daily_histories(&history_range)?;
            log_elapsed!("daily_histories", &stopwatch);
            Ok(DailyHistories { location, histories: daily_histories })
        }
        /// Get the weather history dates for locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations.
        fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
            let locations = self.locations(criteria)?;
            let stopwatch = StopWatch::start_new();
            let mut history_dates = Vec::with_capacity(locations.len());
            for location in locations {
                let inner_stopwatch = StopWatch::start_new();
                let archive = self.get_archive(&location.alias)?;
                let dates = archive.dates()?;
                log_elapsed!(trace, &format!("{} history dates", location.alias), &inner_stopwatch);
                history_dates.push(HistoryDates { location, history_dates: dates.date_ranges })
            }
            log_elapsed!("history_dates", &stopwatch);
            Ok(history_dates)
        }
        /// Get the summary metrics of a locations weather data.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations that should be used.
        fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
            let locations = self.locations(criteria)?;
            let stopwatch = StopWatch::start_new();
            let mut history_summaries = Vec::with_capacity(locations.len());
            for location in locations {
                let archive = self.get_archive(&location.alias)?;
                let summary = archive.summary()?;
                history_summaries.push(HistorySummaries {
                    location,
                    count: summary.count,
                    overall_size: summary.overall_size,
                    raw_size: summary.raw_size,
                    store_size: summary.compressed_size,
                });
            }
            log_elapsed!("history_summaries", &stopwatch);
            Ok(history_summaries)
        }
        fn add_location(&self, location: Location) -> Result<()> {
            let stopwatch = StopWatch::start_new();
            let mut locations = weather_locations(&self.weather_dir)?;
            locations.add(location, &self.weather_dir)?;
            log_elapsed!("add_location", &stopwatch);
            Ok(())
        }
        /// Get the metadata for weather locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations of interest.
        fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
            let stopwatch = StopWatch::start_new();
            let locations = weather_locations(&self.weather_dir)?;
            let locations = locations.as_iter(&criteria.filters, !criteria.icase, criteria.sort).collect();
            log_elapsed!("locations", &stopwatch);
            Ok(locations)
        }
        /// Search for locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` is used to filter the locations search.
        fn search(&self, criteria: LocationCriteria) -> Result<Vec<Location>> {
            search_locations(&self.config, criteria)
        }
    }
}
