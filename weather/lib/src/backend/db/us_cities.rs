//! Encapsulates reading [simple maps](https://simplemaps.com/data/us-cities) US cities
//! CSV database.
use super::*;

/// The default name of the US Cities database;
const DB_NAME: &'static str = "uscities.db";

pub(in crate::backend) use v3::{db_conn, UsCities};
pub(in crate::backend::db) use v3::{load_db, delete_db, info, init_schema};
mod v3 {
    //! The current implementation of the US Cities database API.
    use super::*;
    use crate::admin::entities::UsCitiesInfo;
    use csv::{Reader, StringRecord};
    use rusqlite::{Params, Rows};
    use std::fs::File;

    pub struct UsCities {
        weather_dir: WeatherDir,
        source: PathBuf,
    }
    impl UsCities {
        pub fn new(dirname: &str, filename: &str) -> Result<UsCities> {
            let weather_dir = WeatherDir::try_from(dirname)?;
            let source = PathBuf::from(filename);
            Ok(UsCities { weather_dir, source })
        }
        pub fn search(&self, criteria: LocationCriteria) -> Result<Vec<Location>> {
            let db_file = self.weather_dir.file(DB_NAME);
            match db_file.exists() {
                true => {
                    let conn = db_connection(Some(db_file))?;
                    search(&conn, criteria)
                }
                false => match self.source.exists() {
                    true => {
                        let mut conn = db_connection(None)?;
                        init_schema(&conn)?;
                        load_db(&mut conn, &self.source)?;
                        search(&conn, criteria)
                    }
                    false => {
                        log::warn!("US cities database is not available.");
                        Ok(vec![])
                    }
                },
            }
        }
    }
    impl TryFrom<&Config> for UsCities {
        type Error = Error;
        /// Create the [UsCities] API from a configuration.
        fn try_from(config: &Config) -> std::prelude::v1::Result<Self, Self::Error> {
            UsCities::new(&config.weather_data.directory, &config.us_cities.filename)
        }
    }

    fn db_file(weather_dir: &WeatherDir) -> WeatherFile {
        weather_dir.file(DB_NAME)
    }

    pub fn db_conn(weather_dir: &WeatherDir) -> Result<Connection> {
        db_connection(Some(db_file(weather_dir)))
    }

    /// Queries the US cities database for a location.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `name` is the city name to search for (wildcards allowed).
    /// * `state` is the state name to search for (full or 2 letter name, wildcards allowed).
    /// * `limit` restricts the resulting location matches.
    fn search(conn: &Connection, criteria: LocationCriteria) -> Result<Vec<Location>> {
        match (criteria.name, criteria.state) {
            // select all
            (None, None) => {
                const SQL: &'static str = r#"
                        SELECT name, abrev_state, latitude, longitude, timezone FROM city
                        LIMIT :limit
                        "#;
                let params = named_params! {":limit": criteria.limit};
                query(conn, SQL, params, criteria.limit)
            }
            // select by state
            (None, Some(state)) => {
                const SQL: &'static str = r#"
                        SELECT name, state, abrev_state, latitude, longitude, timezone FROM city
                        WHERE state LIKE :state OR abrev_state LIKE :state
                        ORDER BY abrev_state, name
                        LIMIT :limit
                        "#;
                let params = named_params! {":state": state.replace("*", "%"), ":limit": criteria.limit};
                query(conn, SQL, params, criteria.limit)
            }
            // select by city name
            (Some(name), None) => {
                const SQL: &'static str = r#"
                        SELECT name, abrev_state, latitude, longitude, timezone FROM city
                        WHERE name LIKE :name
                        ORDER BY name, abrev_state
                        LIMIT :limit
                        "#;
                let params = named_params! {":name": name.replace("*", "%"), ":limit": criteria.limit};
                query(conn, SQL, params, criteria.limit)
            }
            // select by city name and state
            (Some(name), Some(state)) => {
                const SQL: &'static str = r#"
                        SELECT name, state, abrev_state, latitude, longitude, timezone FROM city
                        WHERE name LIKE :name AND (state LIKE :state OR abrev_state LIKE :state)
                        ORDER BY name, abrev_state
                        LIMIT :limit
                        "#;
                let params = named_params! {
                    ":name": name.replace("*", "%"),
                    ":state": state.replace("*", "%"),
                    ":limit": criteria.limit
                };
                query(conn, SQL, params, criteria.limit)
            }
        }
    }

    /// Executes a query.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `sql` is the query that will be executed.
    /// * `params` are the parameters used by the query.
    /// * `limit` restricts the resulting location matches.
    fn query(conn: &Connection, sql: &str, params: impl Params, limit: usize) -> Result<Vec<Location>> {
        match conn.prepare_cached(sql) {
            Ok(mut stmt) => match stmt.query(params) {
                Ok(rows) => to_locations(rows, limit),
                Err(err) => {
                    let reason = format!("query failed ({})", err);
                    Err(Error::from(reason))
                }
            },
            Err(err) => {
                let reason = format!("Could not prepare SQL ({}).", err);
                Err(Error::from(reason))
            }
        }
    }

    /// Convert the query rows into a collection of [Location]s.
    ///
    /// # Arguments
    ///
    /// * `rows` the resulting rows of a query.
    /// * `size` is the expected size of the [Location] collection.
    fn to_locations(mut rows: Rows, size: usize) -> Result<Vec<Location>> {
        let mut locations = Vec::with_capacity(size);
        while let Some(row) = rows.next()? {
            let name: String = row.get("name")?;
            let state: String = row.get("abrev_state")?;
            let location = Location {
                name: format!("{}, {}", name, state),
                alias: Default::default(),
                longitude: row.get("longitude")?,
                latitude: row.get("latitude")?,
                tz: row.get("timezone")?,
            };
            locations.push(location);
        }
        Ok(locations)
    }

    /// Initializes the US Cities database table.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    pub fn init_schema(conn: &Connection) -> Result<()> {
        const INIT_SQL: &str = r#"
                BEGIN;
                CREATE TABLE IF NOT EXISTS city
                (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    state TEXT NOT NULL,
                    abrev_state TEXT NOT NULL,
                    latitude TEXT NOT NULL,
                    longitude TEXT NOT NULL,
                    timezone TEXT NOT NULL
                );

                -- cover the name with an index
                CREATE INDEX IF NOT EXISTS idx_city_name ON city(name);

                -- cover the state name with an index
                CREATE INDEX IF NOT EXISTS idx_city_state ON city(state);

                -- cover the abreviated state name with an index
                CREATE INDEX IF NOT EXISTS idx_city_abrev_state ON city(abrev_state);

                COMMIT;
                "#;
        match conn.execute_batch(INIT_SQL) {
            Ok(_) => Ok(()),
            Err(err) => {
                let reason = format!("Error initializing schema ({}).", &err);
                Err(Error::from(reason))
            }
        }
    }

    /// Loads the US Cities `CSV` file into the database.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `path` points to the `CSV` file that will be loaded.
    pub fn load_db(conn: &mut Connection, path: &PathBuf) -> Result<usize> {
        if !path.exists() {
            let reason = format!("Could not file '{}'.", path.display());
            Err(Error::from(reason))
        } else if !path.is_file() {
            let reason = format!("'{}' should be a file.", path.display());
            Err(Error::from(reason))
        } else {
            match Reader::from_path(path) {
                Ok(reader) => insert_cities(conn, reader),
                Err(err) => {
                    let reason = format!("Could not read CSV file '{}' ({}).", path.display(), err);
                    Err(Error::from(reason))
                }
            }
        }
    }

    pub fn delete_db(weather_dir: &WeatherDir) -> Result<()> {
        let file = db_file(weather_dir);
        if !file.exists() {
            Ok(())
        } else {
            match std::fs::remove_file(file.path()) {
                Ok(_) => Ok(()),
                Err(err) => {
                    let reason = format!("Could not delete '{}' ({}).", file, err);
                    Err(Error::from(reason))
                }
            }
        }
    }

    /// Reads the US Cities `CSV` file and inserts each row into the database.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `reader` is used to get each row in the `CSV` file.
    fn insert_cities(conn: &mut Connection, reader: Reader<File>) -> Result<usize> {
        let mut tx = conn.transaction()?;
        let mut count = 0;
        for read_result in reader.into_records() {
            match read_result {
                Ok(record) => {
                    insert_city(&mut tx, record)?;
                    count += 1;
                }
                Err(err) => {
                    let reason = format!("Error reading CSV record ({}).", err);
                    return Err(Error::from(reason));
                }
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Inserts a US City into the database.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `record` is a row from the US Cities `CSV` file that will be added to the database.
    fn insert_city(tx: &mut Transaction, record: StringRecord) -> Result<()> {
        const SQL: &str = r#"
                INSERT INTO city (name, state, abrev_state, latitude, longitude, timezone)
                    VALUES (:name, :state, :abrev_state, :lat, :long, :tz)
                "#;
        let mut stmt = tx.prepare_cached(SQL).unwrap();
        let name = record.get(1).map_or(Default::default(), |v| v.to_string());
        let state = record.get(3).map_or(Default::default(), |v| v.to_string());
        let abrev_state = record.get(2).map_or(Default::default(), |v| v.to_string());
        match stmt.execute(named_params! {
            ":name": &name,
            ":state": &state,
            ":abrev_state": &abrev_state,
            ":lat": record.get(6).map_or(Default::default(), |v| v.to_string()),
            ":long": record.get(7).map_or(Default::default(), |v| v.to_string()),
            ":tz": record.get(13).map_or(Default::default(), |v| v.to_string())
        }) {
            Ok(_) => Ok(()),
            Err(err) => {
                let reason = format!("Error inserting City '{}, {}/{}' ({})", name, state, abrev_state, err);
                Err(Error::from(reason))
            }
        }
    }

    pub fn info(weather_dir: &WeatherDir) -> Result<UsCitiesInfo> {
        let file = db_file(weather_dir);
        if file.exists() {
            let db_size = file.size() as usize;
            let conn = db_connection(Some(file))?;
            let state_info = state_info(&conn)?;
            Ok(UsCitiesInfo { db_size, state_info })
        } else {
            Ok(UsCitiesInfo { db_size: 0, state_info: Vec::with_capacity(0) })
        }
    }
    fn state_info(conn: &Connection) -> Result<Vec<(String, usize)>> {
        const SQL: &str = r#"
                SELECT abrev_state, COUNT(*) FROM city
                GROUP BY abrev_state
                ORDER BY abrev_state
                "#;
        let mut state_info: Vec<(String, usize)> = Vec::with_capacity(50);
        let mut stmt = conn.prepare(SQL)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            state_info.push((row.get(0)?, row.get(1)?));
        }
        Ok(state_info)
    }
}
