//! The database implementation of weather data.

pub(crate) mod admin;
mod archive;
mod document_db;
mod hybrid_db;
mod locations;
mod metadata;
mod normalized_db;
mod query;
pub(crate) mod us_cities;

use super::*;
use crate::admin::admin_entities::DbMode;
use crate::db_conn;
use crate::entities;
use chrono::NaiveDate;
use filesys::{archive_history_collector, ArchiveMd, WeatherArchive, WeatherDir, WeatherFile, WeatherHistoryUpdate};
use rusqlite::{named_params, Connection, Transaction};
use std::cmp;
use toolslib::stopwatch::StopWatch;

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
#[macro_export]
macro_rules! db_conn {
    ($weather_dir:expr) => {
        $crate::backend::db::db_connection(Some($weather_dir.file(DB_FILENAME)))
    };
}

/// Create a [`DataAdapter`] based on the database configuration.
///
/// # Arguments
///
/// `dirname` is the directory containing weather data.
pub(in crate::backend) fn data_adapter(config: Config) -> Result<Box<dyn DataAdapter>> {
    let weather_dir = WeatherDir::try_from(&config)?;
    let conn = db_conn!(&weather_dir)?;
    match admin::database_configuration(&conn)? {
        DbMode::Hybrid => hybrid_db::data_adapter(config, weather_dir),
        DbMode::Document(compressed) => document_db::data_adapter(config, weather_dir, compressed),
        DbMode::Normalized => normalized_db::data_adapter(config, weather_dir),
    }
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
