//! The implementations of weather data.

pub mod db;
pub mod filesys;
mod history;

pub use config::Config;
mod config;

use super::*;
use crate::entities::{
    DailyHistories, DataCriteria, DateRange, History, HistoryDates, HistorySummaries, Location, LocationCriteria,
};
use std::path::{Path, PathBuf};

impl From<rusqlite::Error> for Error {
    /// Add support to convert rusqlite database errors.
    fn from(err: rusqlite::Error) -> Self {
        Error::from(format!("SQL: {}", err))
    }
}

/// Get the backends implementation of weather data.
///
/// # Arguments
///
/// * `config_file` is the weather data configuration filename.
/// * `dirname` is the weather data directory name override.
/// * `no_db` is used to force using the archive implementation of weather data.
pub fn data_api(config_file: Option<PathBuf>, dirname: Option<PathBuf>, no_db: bool) -> Result<DataAPI> {
    let mut config = Config::new(config_file)?;
    if let Some(path) = dirname {
        config.weather_data.directory = path.display().to_string();
    }
    let weather_dir = filesys::WeatherDir::try_from(&config)?;
    let data_adapter = if no_db || db::db_file(&weather_dir).is_none() {
        filesys::data_adapter(config)
    } else {
        db::data_adapter(config)
    }?;
    Ok(DataAPI(data_adapter))
}

pub struct DataAPI(Box<dyn DataAdapter>);
/// The backend API for weather data.
impl DataAPI {
    /// Get the weather data configuration.
    ///
    pub fn get_config(&self) -> &Config {
        &self.0.config()
    }
    /// Add weather data history to a location.
    ///
    /// # Arguments
    ///
    /// - `daily_histories` contains the historical weather data that will be added.
    ///
    pub fn add_daily_histories(&self, daily_histories: DailyHistories) -> Result<usize> {
        self.0.add_daily_histories(&daily_histories)
    }
    /// Get daily weather history for a location.
    ///
    /// It is an error if more than 1 location is found.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the location.
    /// - `history_range` covers the history dates returned.
    ///
    pub fn get_daily_history(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories> {
        let location = self.get_location(&criteria)?;
        self.0.daily_histories(location, history_range)
    }
    /// Get the history dates for locations.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations.
    ///
    pub fn get_history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
        self.0.history_dates(criteria)
    }
    /// Get a summary of location weather data.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations.
    ///
    pub fn get_history_summary(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
        self.0.history_summaries(criteria)
    }
    /// Get the weather location metadata.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations of interest.
    ///
    pub fn get_locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
        self.0.locations(criteria)
    }
    pub fn add_location(&self, location: Location) -> Result<()> {
        self.0.add_location(location)
    }
    /// Search for a location.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the location search criteria.
    ///
    pub fn search_locations(&self, criteria: LocationCriteria) -> Result<Vec<Location>> {
        self.0.search(criteria)
    }
    /// Used internally to get a single location, error otherwise.
    ///
    /// # Arguments
    ///
    /// - `criteria` is the location being searched for.
    ///
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

/// The `API` common to all the backend implementations.
trait DataAdapter {
    /// Get the data adapter configuration.
    ///
    fn config(&self) -> &Config;
    /// Add weather data history for a location.
    ///
    /// # Arguments
    ///
    /// - `histories` has the location and histories to add.
    ///
    fn add_daily_histories(&self, histories: &DailyHistories) -> Result<usize>;
    /// Reports if all histories were added or not.
    ///
    /// # Arguments
    ///
    /// - `histories` is the full collection of weather histories.
    /// - `filesys` is the collection of histories added to the archives.
    /// - `db` is the collection of histories added to the database.
    ///
    fn audit_add_histories(&self, histories: &Vec<History>, filesys: Vec<&History>, db: Vec<&History>) {
        let histories_len = histories.len();
        let filesys_len = filesys.len();
        let db_len = db.len();
        if (histories_len == filesys_len) && (filesys_len == db_len) {
            log::debug!("All histories added.")
        } else {
            log::warn!(
                "{} histories received, {} archive histories added, {} DB histories added.",
                histories_len,
                filesys_len,
                db_len
            )
        }
    }
    /// Returns the daily weather data history for a location.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies what location should be used.
    /// - `history_range` specifies the date range that should be used.
    ///
    fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories>;
    /// Get the weather history dates for locations.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations.
    ///
    fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>>;
    /// Get a summary of the weather history available for locations.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations that should be used.
    ///
    fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>>;
    /// Add a weather data location.
    ///
    /// # Arguments
    ///
    /// - `location` is the location that will be added.
    ///
    fn add_location(&self, location: Location) -> Result<()>;
    /// Get the metadata for weather locations.
    ///
    /// # Arguments
    ///
    /// - `criteria` identifies the locations of interest.
    ///
    fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>>;
    /// Search for locations.
    ///
    /// # Arguments
    ///
    /// - `criteria` is used to filter the locations search.
    ///
    fn search(&self, criteria: LocationCriteria) -> Result<Vec<Location>>;
}

#[cfg(test)]
mod testlib {
    //! A library for common utilities used by the backend.

    use rand::Rng;
    use std::{env, fmt, fs, path};

    /// Used to create a temporary weather directory and delete it as part of the function exit.
    #[derive(Debug)]
    pub(in crate::backend) struct TestFixture(path::PathBuf);
    impl TestFixture {
        /// Creates a test weather directory or panics if a unique directory cannot be created.
        pub(in crate::backend) fn create() -> Self {
            let tmpdir = env::temp_dir();
            let mut weather_dir: Option<path::PathBuf> = None;
            // try to create a test directory 10 times
            for _ in [0..10] {
                let test_dir = tmpdir.join(format!("weather_dir-{}", generate_random_string(15)));
                match test_dir.exists() {
                    true => {
                        eprintln!("Test directory '{}' exists...", test_dir.as_path().display())
                    }
                    false => {
                        weather_dir.replace(test_dir);
                        break;
                    }
                }
            }
            match weather_dir {
                Some(root_dir) => match fs::create_dir(&root_dir) {
                    Ok(_) => Self(root_dir),
                    Err(e) => {
                        panic!("Error creating '{}': {}", root_dir.as_path().display(), e.to_string())
                    }
                },
                None => panic!("Tried 10 times to get a unique test directory name and failed..."),
            }
        }
        pub(in crate::backend) fn copy_resources(&self, source: &path::PathBuf) {
            if source.is_file() {
                let target = self.0.join(source.file_name().unwrap().to_str().unwrap());
                if let Err(err) = fs::copy(source, &target) {
                    panic!("Error copying {} to {} ({}).", source.as_path().display(), self, &err);
                }
            } else {
                let paths = fs::read_dir(&source).unwrap();
                for entry in paths {
                    let source_path = entry.unwrap().path();
                    let target_path = self.0.join(source_path.file_name().unwrap().to_str().unwrap());
                    if let Err(err) = fs::copy(&source_path, &target_path) {
                        panic!("Error copying {} to {} ({}).", source_path.as_path().display(), self, &err);
                    }
                }
            }
        }
    }
    impl Drop for TestFixture {
        /// Clean up the temporary directory as best you can.
        fn drop(&mut self) {
            if let Err(e) = fs::remove_dir_all(self.to_string()) {
                eprintln!("Yikes... Error cleaning up test weather_dir: {}", e.to_string());
            }
        }
    }
    impl fmt::Display for TestFixture {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0.as_path().display())
        }
    }
    impl From<&TestFixture> for path::PathBuf {
        fn from(value: &TestFixture) -> Self {
            path::PathBuf::from(value.to_string())
        }
    }

    pub(in crate::backend) fn generate_random_string(len: usize) -> String {
        let mut rand = rand::thread_rng();
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmonopqrstuvwxyz0123456789";
        let random_string = (0..len)
            .map(|_| {
                let idx = rand.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect();
        // eprintln!("generate_random_string: {}...", random_string);
        random_string
    }

    pub(in crate::backend) fn test_resources() -> path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("tests")
    }
}
