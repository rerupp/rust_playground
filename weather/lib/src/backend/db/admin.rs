//! The weather data administration database API.
use super::{
    db_conn, db_connection, db_file, DB_FILENAME, history, locations, us_cities, DataCriteria, Error, Result,
    WeatherDir, WeatherFile,
};

#[cfg(test)]
use super::testlib;

pub use v4::{db_details, drop_db, init_db, reload, uscities_delete, uscities_info, uscities_load};

mod v4 {
    //! The implementation of weather data administration of a database.
    use super::*;
    use crate::admin::entities::{DbDetails, LocationDetails, UsCitiesInfo};
    use rusqlite::Connection;
    use std::path::PathBuf;

    /// Initialize the database schema.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `db_mode` is the database configuration to initialize.
    /// * `drop` when true will delete the schema before initialization.
    /// * `load` when true will load weather data into the database.
    pub fn init_db(weather_dir: &WeatherDir, drop: bool, load: bool, threads: usize) -> Result<()> {
        if drop {
            drop_db(weather_dir, false)?;
        }
        let mut conn = db_conn!(weather_dir)?;
        init_schema(&conn)?;
        if load {
            log::debug!("loading data");
            locations::load(&mut conn, weather_dir)?;
            history::load(conn, weather_dir, threads)?;
        }
        Ok(())
    }

    /// Deletes the current database schema.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `delete` when true will remove the database file.
    pub fn drop_db(weather_dir: &WeatherDir, delete: bool) -> Result<()> {
        if let Some(db_file) = db_file(weather_dir) {
            if delete {
                delete_db(&db_file)?;
            } else {
                drop_schema(db_connection(Some(db_file))?)?;
            }
        }
        Ok(())
    }

    /// Provide information about the database.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    pub fn db_details(weather_dir: &WeatherDir) -> Result<Option<DbDetails>> {
        let db_details = if let Some(db_file) = db_file(weather_dir) {
            let size = db_file.size() as usize;
            let conn = db_connection(Some(db_file))?;
            let locations_info = locations_details(&conn)?;
            Some(DbDetails { size, location_details: locations_info })
        } else {
            None
        };
        Ok(db_details)
    }

    /// Reload metadata and history for locations.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `criteria` identifies the locations that will be reloaded.
    pub fn reload(weather_dir: &WeatherDir, criteria: DataCriteria) -> Result<Vec<String>> {
        let mut reloaded = Vec::with_capacity(criteria.filters.len());
        if let Some(db_file) = db_file(weather_dir) {
            let mut conn = db_connection(Some(db_file))?;
            for location in locations::get(&conn, &criteria.filters, true)? {
                history::reload(&mut conn, weather_dir, &location.alias)?;
                reloaded.push(location.alias);
            }
        }
        Ok(reloaded)
    }

    /// Creates the database counting the US Cities `CSV` file.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// *`csv_file` is the US Cities `CSV` file to load.
    pub fn uscities_load(weather_dir: &WeatherDir, csv_file: &PathBuf) -> Result<usize> {
        let mut conn = us_cities::db_conn(weather_dir)?;
        us_cities::init_schema(&conn)?;
        let mut stmt = conn.prepare("SELECT COUNT(*) from city")?;
        let count = stmt.query_row([], |row| {
            let count: usize = row.get(0)?;
            Ok(count)
        })?;
        drop(stmt);
        if count == 0 {
            us_cities::load_db(&mut conn, csv_file)
        } else {
            Err(Error::from("US Cities have already been loaded, delete it first."))
        }
    }

    /// Delete the US Cities database.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    pub fn uscities_delete(weather_dir: &WeatherDir) -> Result<()> {
        us_cities::delete_db(weather_dir)
    }

    /// Show information about the US Cities database.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    pub fn uscities_info(weather_dir: &WeatherDir) -> Result<UsCitiesInfo> {
        us_cities::info(weather_dir)
    }

    /// Get weather history information for locations.
    ///
    /// Arguments
    ///
    /// * `conn` is the database connection that will be used.
    fn locations_details(conn: &Connection) -> Result<Vec<LocationDetails>> {
        const SQL: &str = r#"
            SELECT l.alias as alias, SUM(m.size) AS size, COUNT(*) AS histories
            FROM metadata AS m
                INNER JOIN locations AS l ON m.lid = l.id
            GROUP BY alias
            "#;
        let mut stmt = conn.prepare(SQL)?;
        let mut rows = stmt.query([])?;
        let mut location_details = vec![];
        while let Some(row) = rows.next()? {
            location_details.push(LocationDetails {
                alias: row.get("alias")?,
                size: row.get("size")?,
                histories: row.get("histories")?,
            });
        }
        Ok(location_details)
    }

    /// Delete the database file.
    ///
    /// Arguments
    ///
    /// * `db_file` is the database file.
    fn delete_db(db_file: &WeatherFile) -> Result<()> {
        log::debug!("deleting {}", db_file);
        match std::fs::remove_file(db_file.to_string()) {
            Ok(_) => Ok(()),
            Err(err) => {
                let reason = format!("Error deleting database ({}).", &err);
                Err(Error::from(reason))
            }
        }
    }

    /// Initialize the database schema.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `db_mode` is the database configuration.
    fn init_schema(conn: &Connection) -> Result<()> {
        log::debug!("db schema");
        let sql = include_str!("schema.sql");
        match conn.execute_batch(sql) {
            // Ok(_) => init_config(conn),
            Ok(_) => Ok(()),
            Err(err) => {
                let reason = format!("Error initializing schema ({}).", &err);
                Err(Error::from(reason))
            }
        }
    }

    /// Delete the database schema.
    ///
    /// Arguments
    ///
    /// * `conn` is the database connection that will be used.
    fn drop_schema(conn: Connection) -> Result<()> {
        log::debug!("drop schema");
        let sql = include_str!("drop.sql");
        match conn.execute_batch(sql) {
            Ok(_) => match conn.execute("VACUUM", ()) {
                Ok(_) => Ok(()),
                Err(err) => {
                    let reason = format!("Error retrieving disk space from database ({}).", &err);
                    Err(Error::from(reason))
                }
            },
            Err(err) => {
                let reason = format!("Error dropping schema ({})", &err);
                Err(Error::from(reason))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        // use std::path::PathBuf;

        #[test]
        fn admin() {
            let fixture = testlib::TestFixture::create();
            let test_files = testlib::test_resources().join("db");
            fixture.copy_resources(&test_files);
            let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
            let db_file = PathBuf::from(&weather_dir.to_string()).join(DB_FILENAME);
            assert!(!db_file.exists());
            init_db(&weather_dir, false, false, 1).unwrap();
            assert!(db_file.exists());
            db_details(&weather_dir).unwrap().expect("Did not get DbDetails");
            drop_db(&weather_dir, false).unwrap();
        }
    }
}
