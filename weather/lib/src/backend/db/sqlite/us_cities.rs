//! Encapsulates reading [simple maps](https://simplemaps.com/data/us-cities) US cities
//! CSV database.
use super::{commit_tx, create_tx, db_connection, execute_sql, prepare_sql, query_rows, SqlResult};
use crate::admin::UsCityDetails;
use crate::{
    backend::filesys::WeatherDir,
    entities::{Location, LocationCriteria},
};
use csv::Reader;
use rusqlite::{named_params, Connection, Row};
use sql_query_builder as sql;
use std::{fs::File, path::PathBuf};

/// The default name of the US Cities database;
const DB_FILENAME: &'static str = "uscities.db";

/// The database city name column.
const CITY: &'static str = "city";

/// The database full state name column.
const STATE: &'static str = "state";

/// The database 2 letter state name column.
const STATE_ID: &'static str = "state_id";

// The data name column
const NAME: &str = "name";

/// The database city latitude column.
const LATITUDE: &'static str = "latitude";

/// The database city longitude column.
const LONGITUDE: &'static str = "longitude";

/// The database city timezone column.
const TIMEZONE: &'static str = "timezone";

/// Create a database locations specific error message.
macro_rules! error {
    ($($arg:tt)*) => {
        crate::Error::from(format!("US Cities {}", format!($($arg)*)))
    }
}

/// Create an error from the locations specific error message.
macro_rules! err {
    ($($arg:tt)*) => {
        Err(error!($($arg)*))
    };
}

/// Check if the US Cities database exists.
///
pub fn exists(weather_dir: &WeatherDir) -> bool {
    weather_dir.file(DB_FILENAME).exists()
}

/// Open the US Cities database.
///
pub fn open(weather_dir: &WeatherDir) -> crate::Result<Connection> {
    if !exists(weather_dir) {
        err!("{} has not been created.", DB_FILENAME)
    } else {
        match db_connection(Some(weather_dir.file(DB_FILENAME))) {
            Ok(conn) => Ok(conn),
            Err(error) => err!(" could not open {}: {:?}", DB_FILENAME, error),
        }
    }
}

/// Create the US Cities database.
///
pub fn create(weather_dir: &WeatherDir, csv_file: &str) -> crate::Result<()> {
    if exists(weather_dir) {
        err!("{} has already been created.", DB_FILENAME)?;
    }
    let mut stopwatch = toolslib::stopwatch::StopWatch::start_new();
    let mut conn = match db_connection(Some(weather_dir.file(DB_FILENAME))) {
        Ok(conn) => conn,
        Err(error) => err!(" could not create {}: {:?}", DB_FILENAME, error)?,
    };
    init_schema(&mut conn)?;
    log::debug!("database created in {stopwatch}");
    stopwatch.start();
    load_db(&mut conn, PathBuf::from(csv_file))?;
    log::debug!("database loaded in {stopwatch}");
    Ok(())
}

/// Delete the US Cities database
///
pub fn delete(weather_dir: &WeatherDir) -> crate::Result<()> {
    let file = weather_dir.file(DB_FILENAME);
    if file.exists() {
        if let Err(error) = file.remove() {
            err!("failed to delete database file: {:?}", error)?;
        }
    }
    Ok(())
}

pub fn search(conn: &Connection, criteria: LocationCriteria) -> crate::Result<Vec<Location>> {
    // query the location rows
    let query = generate_query(criteria);
    let mut stmt = prepare_sql!(conn, &query, "failed to prepare query");
    let mut rows = query_rows!(stmt, [], "error executing query");

    // create the locations from the rows
    let mut locations = vec![];
    loop {
        match rows.next() {
            Err(error) => err!("failed getting next row: {:?}", error)?,
            Ok(None) => break,
            Ok(Some(row)) => {
                #[inline]
                fn next_location(row_: &Row) -> SqlResult<Location> {
                    let city: String = row_.get(CITY)?;
                    let state_id: String = row_.get(STATE_ID)?;
                    Ok(Location {
                        name: format!("{}, {}", city, state_id),
                        city: row_.get(CITY)?,
                        state_id,
                        state: row_.get(STATE)?,
                        alias: Default::default(),
                        longitude: row_.get(LONGITUDE)?,
                        latitude: row_.get(LATITUDE)?,
                        tz: row_.get(TIMEZONE)?,
                    })
                }
                match next_location(row) {
                    Ok(location) => locations.push(location),
                    Err(error) => err!("error creating location from row: {:?}", error)?,
                }
            }
        }
    }
    Ok(locations)
}

fn generate_query(mut criteria: LocationCriteria) -> String {
    // prepare wildcards for the search
    if let Some(city) = criteria.filter.city.take() {
        criteria.filter.city.replace(city.replace("*", "%"));
    }
    if let Some(state) = criteria.filter.state.take() {
        criteria.filter.state.replace(state.replace("*", "%"));
    }
    if let Some(name) = criteria.filter.name.take() {
        criteria.filter.name.replace(name.replace("*", "%"));
    }

    // build the select statement
    let mut query = sql::Select::new()
        .select(&format!("{CITY}, {STATE}, {STATE_ID}, {NAME}, {LATITUDE}, {LONGITUDE}, {TIMEZONE}"))
        .from("cities");
    let state_column = match &criteria.filter.state {
        None => STATE_ID,
        Some(state) => match state.len() > 2 {
            true => STATE,
            false => STATE_ID,
        }
    };
    match (criteria.filter.city, criteria.filter.state, criteria.filter.name) {
        (Some(city), None, None) => {
            query = query
                .where_clause(&format!("{CITY} LIKE '{city}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        (Some(city), Some(state), None) => {
            query = query
                .where_and(&format!("{CITY} LIKE '{city}'"))
                .where_and(&format!("{state_column} LIKE '{state}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        (Some(city), None, Some(name)) => {
            query = query
                .where_and(&format!("{CITY} LIKE '{city}'"))
                .where_and(&format!("{NAME} LIKE '{name}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        (None, Some(state), None) => {
            query = query
                .where_clause(&format!("{state_column} LIKE '{state}'"))
                .order_by(&format!("{state_column}, {CITY} ASC"));
        }
        (None, Some(state), Some(name)) => {
            query = query
                .where_and(&format!("{state_column} LIKE '{state}'"))
                .where_and(&format!("{NAME} LIKE '{name}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        (None, None, Some(name)) => {
            query = query
                .where_clause(&format!("{NAME} LIKE '{name}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        (Some(city), Some(state), Some(name)) => {
            query = query
                .where_and(&format!("{CITY} LIKE '{city}'"))
                .where_and(&format!("{STATE_ID} LIKE '{state}'"))
                .where_and(&format!("{NAME} LIKE '{name}'"))
                .order_by(&format!("{CITY}, {state_column} ASC"));
        }
        _ => (),
    }
    let query = query.limit(&criteria.limit.to_string()).to_string();
    query
}

/// Initializes the US Cities database table.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used.
fn init_schema(conn: &Connection) -> crate::Result<()> {
    const INIT_SQL: &str = r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS cities
        (
            id INTEGER PRIMARY KEY,
            city TEXT NOT NULL COLLATE NOCASE,
            state TEXT NOT NULL COLLATE NOCASE,
            state_id TEXT NOT NULL COLLATE NOCASE,
            name TEXT GENERATED ALWAYS AS (city || ', ' || state_id),
            latitude TEXT NOT NULL,
            longitude TEXT NOT NULL,
            timezone TEXT NOT NULL
        );

        -- cover the city with an index
        CREATE INDEX IF NOT EXISTS idx_cities_city ON cities(city COLLATE NOCASE);

        -- cover the state name with an index
        CREATE INDEX IF NOT EXISTS idx_cities_state ON cities(state COLLATE NOCASE);

        -- cover the abreviated state name with an index
        CREATE INDEX IF NOT EXISTS idx_cities_state_id ON cities(state_id COLLATE NOCASE);

        -- cover the location name with an index
        CREATE INDEX IF NOT EXISTS idx_cities_name ON cities(name COLLATE NOCASE);

        COMMIT;
    "#;
    if let Err(error) = conn.execute_batch(INIT_SQL) {
        err!("failed to initialize US Cities database schema: {:?}", error)?;
    }
    Ok(())
}

/// Loads the US Cities `CSV` file into the database.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used.
/// * `path` points to the `CSV` file that will be loaded.
fn load_db(conn: &mut Connection, path: PathBuf) -> crate::Result<usize> {
    if !path.exists() {
        err!("source file '{}' was not found.", path.display())
    } else if !path.is_file() {
        err!("'{}' is not a file.", path.display())
    } else {
        match Reader::from_path(&path) {
            Ok(reader) => insert_cities(conn, reader),
            Err(error) => err!("error getting reader for '{}': {:?}", path.display(), error),
        }
    }
}

/// Reads the US Cities `CSV` file and inserts each row into the database.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used.
/// * `reader` is used to get each row in the `CSV` file.
fn insert_cities(conn: &mut Connection, reader: Reader<File>) -> crate::Result<usize> {
    let tx = create_tx!(conn, "failed to create transaction of US Cities load");
    let mut count = 0;
    {
        const SQL: &str = r#"
        INSERT INTO cities (city, state, state_id, latitude, longitude, timezone)
            VALUES (:city, :state, :state_id, :lat, :long, :tz)
        "#;
        let mut stmt = prepare_sql!(tx, SQL, "failed to prepare SQL for US Cities load");
        for read_result in reader.into_records() {
            match read_result {
                Err(error) => err!("error reading CSV record ({}).", error)?,
                Ok(record) => {
                    let params = named_params! {
                        ":city": record.get(1).map_or(Default::default(), |v| v.to_string()),
                        ":state": record.get(3).map_or(Default::default(), |v| v.to_string()),
                        ":state_id": record.get(2).map_or(Default::default(), |v| v.to_string()),
                        ":lat": record.get(6).map_or(Default::default(), |v| v.to_string()),
                        ":long": record.get(7).map_or(Default::default(), |v| v.to_string()),
                        ":tz": record.get(13).map_or(Default::default(), |v| v.to_string())
                    };
                    execute_sql!(stmt, params, "failed to insert US Cities record into DB");
                    count += 1;
                }
            }
        }
    }
    commit_tx!(tx, "failed to commit US Cities load");
    Ok(count)
}

pub fn db_metrics(weather_dir: &WeatherDir) -> crate::Result<UsCityDetails> {
    let file = weather_dir.file(DB_FILENAME);
    if file.exists() {
        let db_size = file.size() as usize;
        let conn = match db_connection(Some(file)) {
            Ok(conn) => conn,
            Err(error) => err!("did not get a db connection: {:?}", error)?,
        };
        let state_info = state_metrics(&conn)?;
        Ok(UsCityDetails { db_size, state_info })
    } else {
        Ok(UsCityDetails { db_size: 0, state_info: Vec::with_capacity(0) })
    }
}

fn state_metrics(conn: &Connection) -> crate::Result<Vec<(String, usize)>> {
    // get the state summary
    let query = sql::Select::new()
        .select(&format!("{STATE_ID}, COUNT(*)"))
        .from("cities")
        .group_by(STATE_ID)
        .order_by(STATE_ID)
        .to_string();
    let mut stmt = prepare_sql!(conn, &query, "failed to prepare metrics query");
    let mut rows = query_rows!(stmt, [], "failed to query metrics");

    // process the query results
    let mut state_info: Vec<(String, usize)> = Vec::with_capacity(52);
    loop {
        match rows.next() {
            Err(error) => err!("failed to get next metrics row: {:?}", error)?,
            Ok(None) => break,
            Ok(Some(row)) => {
                #[inline]
                fn next_state_count(row_: &Row) -> SqlResult<(String, usize)> {
                    Ok((row_.get(0)?, row_.get(1)?))
                }
                match next_state_count(row) {
                    Ok(state_count) => state_info.push(state_count),
                    Err(error) => err!("failed to get state metrics: {:?}", error)?,
                }
            }
        }
    }
    Ok(state_info)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::backend::testlib;
//
//     #[test]
//     fn sql() {
//         println!("{}", generate_query(LocationCriteria { name: None, state: None, limit: 5 }));
//         println!("{}", generate_query(LocationCriteria { name: Some("city*".to_string()), state: None, limit: 5 }));
//         println!("{}", generate_query(LocationCriteria { name: None, state: Some("*s".to_string()), limit: 5 }));
//         println!(
//             "{}",
//             generate_query(LocationCriteria {
//                 name: Some("*city*".to_string()),
//                 state: Some("*s*".to_string()),
//                 limit: 5
//             })
//         )
//     }
//
//     #[test]
//     fn full_monty() {
//         let fixture = testlib::TestFixture::create();
//         let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
//         assert!(!exists(&weather_dir));
//         create(&weather_dir, "../../../uscities.csv").unwrap();
//         let conn = open(&weather_dir).unwrap();
//         let testcase = search(&conn, LocationCriteria { name: None, state: None, limit: 5 }).unwrap();
//         assert_eq!(testcase.len(), 5);
//         let metrics = db_metrics(&weather_dir).unwrap();
//         assert_eq!(metrics.state_info.len(), 52);
//         println!("{:?}", metrics);
//     }
// }
