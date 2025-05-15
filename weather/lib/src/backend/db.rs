//! The database implementation of weather data.

pub(crate) mod admin;
mod archive;
mod locations;
mod metadata;
mod query;
pub(crate) mod us_cities;
mod history;

use super::*;
use crate::entities;
use filesys::{WeatherDir, WeatherFile, WeatherHistoryUpdate};
use rusqlite::{named_params, Connection, Transaction};

/// The name of the database
const DB_FILENAME: &str = "weather_data.db";

/// Create a database connection.
///
/// # Arguments
///
/// * `optional_file` is the database file, if `None` an in-memory database will be used.
pub fn db_connection(optional_file: Option<WeatherFile>) -> Result<Connection> {
    match optional_file {
        Some(file) => Ok(Connection::open(file.to_string())?),
        None => Ok(Connection::open_in_memory()?),
    }
}

/// A helper to create a database connection.
// move this outside the version
macro_rules! db_conn {
    ($weather_dir:expr) => {
        $crate::backend::db::db_connection(Some($weather_dir.file(DB_FILENAME)))
    };
}
use db_conn;

/// Create a [`DataAdapter`] based on the database configuration.
///
/// # Arguments
///
/// `dirname` is the directory containing weather data.
pub(in crate::backend) fn data_adapter(config: Config) -> Result<Box<dyn DataAdapter>> {
    log::debug!("Database data adapter");
    let weather_dir = WeatherDir::try_from(&config)?;
    let data_adapter = DbDataAdapter { config, weather_dir };
    Ok(Box::new(data_adapter))
}

/// Get the size estimate of a table in the database. This is specific to `sqlite`.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used.
/// * `table` is the database table name.
fn size_estimate(conn: &Connection, table: &str) -> Result<usize> {
    // the primary id will always be 64 bit
    let mut size_estimate = 0;
    // this is specific to sqlite3
    conn.pragma(None, "table_info", table, |row| {
        let name: String = row.get("name")?;
        let column_type: String = row.get("type")?;
        match column_type.as_str() {
            "REAL" => size_estimate += 8,
            "INTEGER" => {
                if name.ends_with("_t") {
                    size_estimate += 8;
                } else if name == "id" || name == "mid" {
                    // primary ids are always 8 bytes
                    size_estimate += 8;
                } else {
                    size_estimate += 4;
                }
            }
            "TEXT" => (),
            "BLOB" => (), // history documents are small enough it should just be the byte len
            _ => {
                eprintln!("Yikes!!!! Did not recognize column {} type '{}'...", name, column_type);
            }
        }
        Ok(())
    })?;
    Ok(size_estimate)
}

/// Get the weather history database file.
///
/// # Arguments
///
/// * `weather_dir` is the weather data directory.
pub(crate) fn db_file(weather_dir: &WeatherDir) -> Option<WeatherFile> {
    let file = weather_dir.file(DB_FILENAME);
    match file.exists() {
        true => Some(file),
        false => {
            log::info!("WeatherData is not configured to use a database.");
            None
        }
    }
}

/// The database data adapter implementation.
struct DbDataAdapter {
    config: Config,
    /// The weather data directory.
    weather_dir: WeatherDir,
}
impl DataAdapter for DbDataAdapter {
    /// Get the normalized data adapter configuration.
    fn config(&self) -> &Config {
        &self.config
    }

    /// Add weather data history for a location.
    ///
    /// # Arguments
    ///
    /// * `daily_histories` contains the location histories to add.
    fn add_daily_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
        let mut conn = db_conn!(&self.weather_dir)?;
        history::add(&mut conn, &self.weather_dir, daily_histories)
    }

    /// Returns the daily weather data history for a location.
    ///
    /// # Arguments
    ///
    /// * `location` identifies what location should be used.
    /// * `history_range` specifies the date range that should be used.
    fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories> {
        let mut conn = db_conn!(&self.weather_dir)?;
        history::get(&mut conn, location, date_range)
    }

    /// Get the weather history dates for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations.
    fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
        let conn = db_conn!(&self.weather_dir)?;
        query::history_dates(&conn, criteria)
    }

    /// Get a summary of the weather history available for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations that should be used.
    fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
        let mut conn = db_conn!(&self.weather_dir)?;
        history::summary(&mut conn, &self.weather_dir, criteria)
    }

    /// Add a location into the database.
    ///
    /// # Arguments
    ///
    /// * `location` is the location that will be added.
    fn add_location(&self, location: Location) -> Result<()> {
        let mut conn = db_conn!(&self.weather_dir)?;
        locations::add(&mut conn, location, &self.weather_dir)
    }

    /// Get the metadata for weather locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations of interest.
    fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
        let conn = db_conn!(&self.weather_dir)?;
        locations::get(&conn, &criteria.filters, criteria.sort)
    }

    /// Search for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` is used to filter the locations search.
    fn search(&self, criteria: LocationCriteria) -> Result<Vec<Location>> {
        locations::search(&self.config, criteria)
    }
}
// }