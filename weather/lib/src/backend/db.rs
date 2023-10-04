//! The database implementation of weather data.

use crate::{
    backend::{self, history, DataAdapter, Error, Result},
    entities,
};
use rusqlite::Connection;
use toolslib::stopwatch::StopWatch;

// Since database functionality is scoped to this module it's okay to add the error handler
// here and not in the module where Error is defined.
impl From<rusqlite::Error> for Error {
    /// Add support to convert rusqlite database errors.
    fn from(err: rusqlite::Error) -> Self {
        Error::from(format!("sql: {}", err))
    }
}

/// Export the function that will create the database [DataAdapter] adapter.
pub(crate) use v1::{admin, data_adapter};
mod v1 {
    //! The first version of the database implementation.
    use super::*;
    use backend::filesys::{weather_dir, ArchiveMd, WeatherArchive, WeatherDir, WeatherFile};

    #[cfg(test)]
    use backend::testlib;

    /// The name of the database
    const DB_FILENAME: &str = "weather_data.db";

    /// Create a connection to some database.
    ///
    /// # Arguments
    ///
    /// * `optional_path` is the database pathname. If `None` an in-memory database will be used.
    fn db_connection(optional_path: Option<&str>) -> Result<Connection> {
        let conn = match optional_path {
            Some(path) => Connection::open(path)?,
            None => Connection::open_in_memory()?,
        };
        Ok(conn)
    }

    /// A helper to create a database connection.
    macro_rules! db_conn {
        ($weather_dir:expr) => {
            db_connection(Some($weather_dir.file(DB_FILENAME).to_string().as_str()))
        };
    }

    /// Create a [`DataAdapter`] based on the database configuration.
    ///
    /// # Arguments
    ///
    /// `dirname` is the directory containing weather data.
    pub(crate) fn data_adapter(dirname: &str) -> Result<Box<dyn DataAdapter>> {
        let weather_dir = weather_dir(dirname)?;
        let conn = db_conn!(&weather_dir)?;
        let db_config = admin::database_configuration(&conn)?;
        if db_config.hybrid {
            hybrid::data_adapter(weather_dir)
        } else if db_config.document {
            document::data_adapter(weather_dir)
        } else {
            normalized::data_adapter(weather_dir)
        }
    }

    pub(crate) mod admin {
        //! The implementation of weather data adminstation of a database.

        use super::*;
        use backend::filesys::WeatherFile;
        use entities::{DbConfig, DbInfo};
        use rusqlite::named_params;

        /// Initialize the database schema.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory.
        /// * `db_config` is the database configuration.
        /// * `drop` when true will delete the schema before intialization.
        /// * `load` when true will load weather data into the database.
        pub(crate) fn init_db(
            weather_dir: &WeatherDir,
            db_config: DbConfig,
            drop: bool,
            load: bool,
            threads: usize,
        ) -> Result<()> {
            if drop {
                drop_db(weather_dir, false)?;
            }
            let mut conn = db_conn!(weather_dir)?;
            init_schema(&conn, &db_config)?;
            if load {
                log::debug!("loading data");
                locations::load(&mut conn, weather_dir)?;
                if db_config.hybrid {
                    hybrid::load(&mut conn, weather_dir)?;
                } else if db_config.document {
                    document::load(weather_dir, db_config.compress, threads)?;
                } else {
                    normalized::load(weather_dir, threads)?;
                }
            }
            Ok(())
        }

        /// Deletes the current database schema.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory.
        /// * `delete` when true will remove the database file.
        pub(crate) fn drop_db(weather_dir: &WeatherDir, delete: bool) -> Result<()> {
            let db_file = weather_dir.file(DB_FILENAME);
            if db_file.exists() {
                if delete {
                    delete_db(&db_file)?;
                } else {
                    drop_schema(db_conn!(weather_dir)?)?;
                }
            }
            Ok(())
        }

        /// Provide information about the database.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory.
        pub(crate) fn stat(weather_dir: &WeatherDir) -> Result<DbInfo> {
            let file = weather_dir.file(DB_FILENAME);
            let db_info = if file.exists() {
                let size = file.size() as usize;
                let conn = db_conn!(weather_dir)?;
                let config = database_configuration(&conn)?;
                DbInfo { config: Some(config), size }
            } else {
                DbInfo { config: None, size: 0 }
            };
            Ok(db_info)
        }

        /// Get the database configuration.
        ///
        /// Arguments
        ///
        /// * `conn` is the database connection that will be used.
        pub(crate) fn database_configuration(conn: &Connection) -> Result<DbConfig> {
            let mut stmt = conn.prepare("SELECT hybrid, document, full, compress FROM config")?;
            let db_config = stmt.query_row([], |row| {
                let db_config = if row.get("hybrid")? {
                    DbConfig::hybrid()
                } else if row.get("document")? {
                    DbConfig::document(row.get("compress")?)
                } else {
                    DbConfig::normalize()
                };
                Ok(db_config)
            })?;
            Ok(db_config)
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

        /// Insert the database configuration.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `db_config` is the database configuration.
        fn init_config(conn: &Connection, db_config: &DbConfig) -> Result<()> {
            log::debug!("db tables");
            const SQL: &str = r#"
            INSERT INTO config (hybrid, document, full, compress)
                VALUES (:hybrid, :document, :normalize, :compress)
            "#;
            let params = named_params! {
                ":hybrid": db_config.hybrid,
                ":document": db_config.document,
                ":normalize": db_config.normalize,
                ":compress": db_config.compress
            };
            match conn.execute(SQL, params) {
                Ok(_) => Ok(()),
                Err(err) => {
                    let reason = format!("Error setting config table ({}).", &err);
                    Err(Error::from(reason))
                }
            }
        }

        /// Initialize the database schema.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `db_config` is the database configuration.
        fn init_schema(conn: &Connection, db_config: &DbConfig) -> Result<()> {
            log::debug!("db schema");
            let sql = include_str!("db/schema.sql");
            match conn.execute_batch(sql) {
                Ok(_) => init_config(conn, db_config),
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
            let sql = include_str!("db/drop.sql");
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
            use std::path::PathBuf;

            #[test]
            fn admin() {
                let fixture = testlib::TestFixture::create();
                let test_files = testlib::test_resources().join("db");
                fixture.copy_resources(&test_files);
                let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
                let db_file = PathBuf::from(&weather_dir.to_string()).join(DB_FILENAME);
                assert!(!db_file.exists());
                macro_rules! assert_config {
                    ($config:expr, $hybrid:expr, $document:expr, $normalize:expr, $compress:expr) => {
                        assert_eq!($config.hybrid, $hybrid);
                        assert_eq!($config.document, $document);
                        assert_eq!($config.normalize, $normalize);
                        assert_eq!($config.compress, $compress);
                    };
                }
                // hybrid
                admin::init_db(&weather_dir, DbConfig::hybrid(), true, true, 1).unwrap();
                assert!(db_file.exists());
                let config = admin::stat(&weather_dir).unwrap().config.unwrap();
                assert_config!(config, true, false, false, false);
                admin::drop_db(&weather_dir, true).unwrap();
                assert!(!db_file.exists());
                // document uncompressed
                admin::init_db(&weather_dir, DbConfig::document(false), false, true, 1).unwrap();
                assert!(db_file.exists());
                let config = admin::stat(&weather_dir).unwrap().config.unwrap();
                assert_config!(config, false, true, false, false);
                // document compressed
                admin::drop_db(&weather_dir, false).unwrap();
                admin::init_db(&weather_dir, DbConfig::document(true), false, true, 1).unwrap();
                assert!(db_file.exists());
                let config = admin::stat(&weather_dir).unwrap().config.unwrap();
                assert_config!(config, false, true, false, true);
                // full
                admin::drop_db(&weather_dir, false).unwrap();
                admin::init_db(&weather_dir, DbConfig::normalize(), false, false, 1).unwrap();
                assert!(db_file.exists());
                let config = admin::stat(&weather_dir).unwrap().config.unwrap();
                assert_config!(config, false, false, true, false);
            }
        }
    }

    mod query {
        //! The common weather database queries.

        use super::*;
        use chrono::NaiveDate;
        use entities::{DataCriteria, DateRange, History, HistoryDates, HistorySummaries};
        use rusqlite::named_params;
        use std::collections::HashSet;

        /// For a given collection of histories, filter out the ones that already exist.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection to use.
        /// * `lid` is the location id.
        /// * `histories` is what will be filtered.
        pub fn histories_not_found<'h>(
            conn: &Connection,
            lid: i64,
            histories: &'h Vec<History>,
        ) -> Result<Vec<&'h History>> {
            const SQL: &str = r#"SELECT date FROM metadata WHERE lid = :lid"#;
            let stopwatch = StopWatch::start_new();
            let mut stmt = conn.prepare(&SQL)?;
            let mut rows = stmt.query(named_params! {":lid": lid})?;
            let mut existing_dates = HashSet::new();
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get("date")?;
                existing_dates.insert(date);
            }
            let not_found = histories
                .iter()
                .filter_map(|h| match existing_dates.contains(&h.date) {
                    true => {
                        log::warn!("location id={} history already has {}.", lid, h.date);
                        None
                    }
                    false => Some(h),
                })
                .collect();
            log::debug!("histories_not_found {}", stopwatch);
            Ok(not_found)
        }
        /// Get the location history dates.
        ///
        /// # Arguments
        ///
        /// * `conn` is the datbase connection that will be used.
        /// * `criteria` is the location data criteria.
        pub fn history_dates(conn: &Connection, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
            // collect up the locations that match the criteria
            let mut history_dates: Vec<HistoryDates> = locations::get(conn, &criteria.filters, criteria.sort)?
                .into_iter()
                .map(|location| HistoryDates { location, history_dates: vec![] })
                .collect();
            // if the data criteria didn't match anything don't bother with a query
            if history_dates.len() > 0 {
                // collect the location aliases
                let aliases = if criteria.filters.len() > 0 {
                    history_dates.iter().map(|history| history.location.alias.as_str()).collect()
                } else {
                    vec![]
                };
                // now knit in the location history dates
                for (alias, dates) in query_history_dates(conn, aliases)? {
                    for history in &mut history_dates {
                        if history.location.alias == alias {
                            history.history_dates = DateRange::from_dates(dates);
                            break;
                        }
                    }
                }
            }
            Ok(history_dates)
        }

        /// Execute the query to get location history dates.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection used to execute the query.
        /// * `aliases` is used to restrict what locations will be returned.
        fn query_history_dates(conn: &Connection, aliases: Vec<&str>) -> Result<Vec<(String, Vec<NaiveDate>)>> {
            // build the history date query
            let mut sql = r#"
            SELECT
                l.alias AS alias,
                m.date AS date
            FROM locations AS l
                INNER JOIN metadata AS m ON l.id = m.lid
            "#
            .to_string();
            if aliases.len() > 0 {
                let aliases: Vec<String> = aliases.iter().map(|a| format!("'{}'", *a)).collect();
                sql.push_str(&format!("WHERE l.alias IN ({})\n", aliases.join(",")));
            }
            sql.push_str("ORDER BY l.alias, m.date");
            // execute the query
            log::trace!("{}", sql);
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            // now collect the location dates
            let mut location_dates: Vec<(String, Vec<NaiveDate>)> = vec![];
            while let Some(row) = rows.next()? {
                let alias: String = row.get("alias")?;
                let date: NaiveDate = row.get("date")?;
                match location_dates.last_mut() {
                    Some((current_alias, dates)) => {
                        if current_alias == &alias {
                            dates.push(date);
                        } else {
                            location_dates.push((alias, vec![date]));
                        }
                    }
                    None => location_dates.push((alias, vec![date])),
                }
            }
            Ok(location_dates)
        }

        /// Get location history summaries.
        ///
        /// # Arguments
        ///
        /// * `conn` is the datbase connection that will be used.
        /// * `criteria` is the location data criteria.
        pub fn history_summaries(conn: &Connection, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
            let mut history_summaries: Vec<HistorySummaries> = locations::get(conn, &criteria.filters, criteria.sort)?
                .into_iter()
                .map(|location| HistorySummaries {
                    location,
                    count: 0,
                    overall_size: None,
                    raw_size: None,
                    store_size: None,
                })
                .collect();
            let aliases: Vec<&str> = history_summaries.iter().map(|h| h.location.alias.as_str()).collect();
            for (alias, count, compressed_size, raw_size) in query_history_summaries(conn, aliases)? {
                for history in &mut history_summaries {
                    if history.location.alias == alias {
                        history.count = count;
                        history.raw_size = Some(raw_size);
                        history.store_size = Some(compressed_size);
                        break;
                    }
                }
            }
            Ok(history_summaries)
        }

        /// Execute the query to get location history summarries.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection used to execute the query.
        /// * `aliases` is used to restrict what locations will be returned.
        fn query_history_summaries(
            conn: &Connection,
            aliases: Vec<&str>,
        ) -> Result<Vec<(String, usize, usize, usize)>> {
            // build the history summary query
            let mut sql = r#"
            SELECT
                l.alias AS alias,
                COUNT(m.date) AS count,
                SUM(m.store_size) AS store_size,
                SUM(m.size) AS raw_size
            FROM locations AS l
                INNER JOIN metadata AS m ON l.id = m.lid
            "#
            .to_string();
            if aliases.len() > 0 {
                let aliases: Vec<String> = aliases.iter().map(|a| format!("'{}'", a)).collect();
                sql.push_str(&format!("WHERE l.alias IN ({})\n", aliases.join(",")));
            }
            sql.push_str("GROUP BY l.alias\n");
            sql.push_str("ORDER BY l.name");
            // execute the query
            log::trace!("{}", sql);
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            let mut history_summaries: Vec<(String, usize, usize, usize)> = vec![];
            while let Some(row) = rows.next()? {
                let alias: String = row.get("alias")?;
                let count: usize = row.get("count")?;
                let store_size: usize = row.get("store_size")?;
                let raw_size: usize = row.get("raw_size")?;
                history_summaries.push((alias, count, store_size, raw_size));
            }
            Ok(history_summaries)
        }

        /// Calculate the amount of space being used by a table.
        ///
        /// This is terribly expensive but it suffices for right now
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `table_name` is the table whose space will be calculated.
        pub(super) fn table_size(conn: &Connection, table_name: &str) -> Result<Vec<(String, usize)>> {
            // get the count of history dates for each location
            let (total, history_counts) = query::history_counts(conn)?;
            // get the overall size of history and metadata for the table
            let table_size = query::sqlite_history_size(conn, table_name)?;
            eprintln!("'{}' size: {}", table_name, table_size);
            // calculate the sizes based on the number of histories
            let locations_size: Vec<(String, usize)> = history_counts
                .into_iter()
                .map(|(alias, count)| {
                    let percentage = count as f64 / total as f64;
                    let size = (table_size as f64 * percentage) as usize;
                    (alias, size)
                })
                .collect();
            Ok(locations_size)
        }

        /// Used internally to help calculate the amount of history space being used by locations.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `table_name` is the table name that will be examined.
        fn sqlite_history_size(conn: &Connection, table_name: &str) -> Result<usize> {
            const SQL: &str = r#"
            SELECT
                SUM(pgsize) AS size
            FROM dbstat
                WHERE name LIKE :table OR name LIKE '%metadata%'
            "#;
            let mut stmt = conn.prepare(SQL)?;
            let db_size = stmt.query_row(named_params! {":table": format!("%{}%", table_name)}, |row| {
                let size: usize = row.get("size")?;
                Ok(size)
            })?;
            Ok(db_size)
        }

        /// Used internally to help calculate the amount of history space being used by locations.
        ///
        /// # Arguments
        ///
        /// * `conn` is the connection that will be used.
        /// * `table_name` is the table name that will be examined.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        fn history_counts(conn: &Connection) -> Result<(usize, Vec<(String, usize)>)> {
            const SQL: &str = r#"
            SELECT
                l.alias AS alias,
                COUNT(m.date) AS COUNT
            FROM locations AS l
                INNER JOIN metadata AS m ON l.id=m.lid
            GROUP BY l.alias
            ORDER BY l.alias
            "#;
            let mut total: usize = 0;
            let mut counts: Vec<(String, usize)> = vec![];
            let mut stmt = conn.prepare(SQL)?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let alias: String = row.get("alias")?;
                let count: usize = row.get("count")?;
                counts.push((alias, count));
                total += count;
            }
            Ok((total, counts))
        }
    }

    mod locations {
        //! Provide database support for weather data locations.
        use super::*;
        use entities::Location;
        use rusqlite::named_params;

        /// Loads location into the database.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `weather_dir` is the weather data directory.
        pub(super) fn load(conn: &mut Connection, weather_dir: &WeatherDir) -> Result<()> {
            log::debug!("  locations");
            let locations = backend::filesys::weather_locations(weather_dir)?;
            let tx = conn.transaction()?;
            const SQL: &str =
                r#"INSERT INTO locations (name, alias, longitude, latitude, tz) VALUES (?1, ?2, ?3, ?4, ?5)"#;
            {
                // scope the statement to this block allowing it to go out of scope before the commit
                let mut stmt = tx.prepare(SQL)?;
                for location in locations.as_iter(&Vec::with_capacity(0), false, false) {
                    let alias = location.alias.clone();
                    let params = (location.name, location.alias, location.longitude, location.latitude, location.tz);
                    match stmt.execute(params) {
                        Ok(_) => (),
                        Err(err) => {
                            let reason = format!("Error adding location '{}' ({}).", alias, &err);
                            return Err(Error::from(reason));
                        }
                    }
                }
            }
            tx.commit()?;
            Ok(())
        }

        /// Get locations.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `filters` determines what locations will be returned.
        /// * `sort` when true the returned collection will ordered by location name.
        pub(super) fn get(conn: &Connection, filters: &Vec<String>, sort: bool) -> Result<Vec<Location>> {
            let mut stmt = conn.prepare(&query_sql(filters, sort))?;
            let mut rows = stmt.query([])?;
            let mut locations = vec![];
            while let Some(row) = rows.next()? {
                let location = Location {
                    name: row.get("name")?,
                    alias: row.get("alias")?,
                    longitude: row.get("longitude")?,
                    latitude: row.get("latitude")?,
                    tz: row.get("tz")?,
                };
                locations.push(location);
            }
            Ok(locations)
        }

        /// Used internally to generate the `SQL` to select locations.
        ///
        /// # Arguments
        ///
        /// * `filters` has the location names or aliases that restrict what locations are returned.
        /// * `sort` when true will sort the returned collection by location name.
        fn query_sql(filters: &Vec<String>, sort: bool) -> String {
            macro_rules! condition {
                ($filter:expr) => {
                    or!([like!("name", $filter), like!("alias", $filter)])
                };
            }
            let mut sql = r#"
            SELECT
                name, alias, longitude, latitude, tz
            FROM locations
            "#
            .to_string();
            match filters.len() {
                0 => (),
                1 => {
                    let filter = &filters[0];
                    if filter != "*" {
                        sql.push_str(&format!(" WHERE {}", condition!(filter)));
                    }
                }
                _ => {
                    if filters.iter().find(|filter| *filter == "*").is_none() {
                        let conditions: Vec<Condition> = filters.iter().map(|filter| condition!(filter)).collect();
                        sql.push_str(&format!(" WHERE {}", or!(conditions)));
                    }
                }
            }
            if sort {
                sql.push_str(" ORDER BY name ASC");
            }
            log::trace!("{}", sql);
            sql
        }

        /// Get the location id and alias.
        ///
        /// # Arguments
        ///
        /// * `conn` is the datbase connection that will be used.
        pub(super) fn id_aliases(conn: &Connection) -> Result<Vec<(i64, String)>> {
            let mut stmt = conn.prepare("SELECT id, alias FROM locations")?;
            let mut rows = stmt.query([])?;
            let mut id_aliases = vec![];
            while let Some(row) = rows.next()? {
                let id = row.get(0)?;
                let alias = row.get(1)?;
                id_aliases.push((id, alias))
            }
            Ok(id_aliases)
        }

        pub(super) fn location_id(conn: &Connection, alias: &str) -> Result<i64> {
            let mut stmt = conn.prepare("SELECT id FROM locations AS l where l.alias = :alias")?;
            match stmt.query_row(named_params! {":alias": alias}, |row| Ok(row.get(0))) {
                Ok(id) => Ok(id.unwrap()),
                Err(err) => {
                    let reason = format!("Error getting id for '{}' ({}).", alias, err);
                    Err(Error::from(reason))
                }
            }
        }

        pub use conditions::{between, equals, like, or, Between, Condition, Equals, Like, Or};
        mod conditions {
            //! Helpers that dynamically build `SQL WHERE` clause conditions.
            use super::*;
            use std::fmt::Display;

            /// The supported `SQL` conditions.
            #[derive(Debug)]
            pub enum Condition {
                /// A `SQL` *equals* condition.
                #[allow(unused)]
                Equals(Equals),
                /// A `SQL` *like* condition.
                Like(Like),
                /// A `SQL` *or* condition.
                Or(Or),
                /// A `SQL` *between* condition.
                #[allow(unused)]
                Between(Between),
            }
            impl Display for Condition {
                /// Allow the condition to be converted to a string.
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Condition::Equals(equals) => {
                            write!(f, "{}", equals)?;
                        }
                        Condition::Like(like) => {
                            write!(f, "{}", like)?;
                        }
                        Condition::Or(or) => {
                            write!(f, "{}", or)?;
                        }
                        Condition::Between(between) => {
                            write!(f, "{}", between)?;
                        }
                    }
                    Ok(())
                }
            }

            #[derive(Debug, Default)]
            pub struct Equals {
                column: String,
                value: Option<String>,
            }
            #[allow(unused)]
            impl Equals {
                pub fn column(name: &str) -> Self {
                    Self { column: name.to_string(), value: None }
                }
                pub fn value(mut self, value: &str) -> Self {
                    self.value.replace(value.to_string());
                    self
                }
            }
            impl Display for Equals {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    let value = if let Some(ref value) = self.value { value.as_str() } else { "???" };
                    write!(f, "{} = {}", self.column, value)?;
                    Ok(())
                }
            }

            #[macro_export]
            macro_rules! equals {
                ($column:expr, $value:expr) => {
                    Condition::Equals(Equals::column($column).value($value))
                };
            }
            pub use equals;

            #[derive(Debug, Default)]
            pub struct Like {
                column: String,
                value: Option<String>,
            }
            impl Like {
                pub fn column(name: &str) -> Self {
                    Self { column: name.to_string(), value: None }
                }
                pub fn value(mut self, value: &str) -> Self {
                    let mut value = value.replace("*", "%");
                    if !(value.starts_with("'") && value.ends_with("'")) {
                        value = format!("'{}'", value)
                    }
                    self.value.replace(value);
                    self
                }
            }
            impl Display for Like {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    let value = if let Some(ref value) = self.value { value.as_str() } else { "???" };
                    write!(f, "{} LIKE {}", self.column, value)?;
                    Ok(())
                }
            }

            #[macro_export]
            macro_rules! like {
                ($column:expr, $value:expr) => {
                    Condition::Like(Like::column($column).value($value))
                };
            }
            pub use like;

            #[derive(Debug, Default)]
            pub struct Between {
                column: String,
                from: Option<String>,
                thru: Option<String>,
            }
            #[allow(unused)]
            impl Between {
                pub fn column(name: &str) -> Self {
                    Self { column: name.to_string(), from: None, thru: None }
                }
                pub fn from(mut self, from: &str) -> Self {
                    self.from.replace(from.to_string());
                    self
                }
                pub fn thru(mut self, thru: &str) -> Self {
                    self.thru.replace(thru.to_string());
                    self
                }
            }
            impl Display for Between {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    let from = if let Some(ref from) = self.from { from.as_str() } else { "???" };
                    let thru = if let Some(ref thru) = self.thru { thru.as_str() } else { "???" };
                    write!(f, "{} BETWEEN {} AND {}", self.column, from, thru)
                }
            }

            #[macro_export]
            macro_rules! between {
                ($column:expr, $from:expr, $thru:expr) => {
                    Condition::Between(Between::column($column).from($from).thru($thru))
                };
            }
            pub use between;

            #[derive(Debug, Default)]
            pub struct Or(
                // the or conditions
                Vec<Condition>,
            );
            impl Or {
                pub fn conditions(filters: impl IntoIterator<Item = Condition>) -> Self {
                    Self(filters.into_iter().collect())
                }
            }
            impl Display for Or {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    if self.0.len() > 0 {
                        let condition = &self.0[0];
                        write!(f, "({}", condition)?;
                        for condition in self.0[1..].iter() {
                            write!(f, " OR {}", condition)?;
                        }
                        write!(f, ")")?;
                    }
                    Ok(())
                }
            }

            #[macro_export]
            macro_rules! or {
                ($conditions:expr) => {
                    Condition::Or(Or::conditions($conditions))
                };
            }
            pub use or;

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn equals() {
                    assert_eq!(Equals::column("name").to_string(), "name = ???");
                    assert_eq!(Equals::column("name").value("'value'").to_string(), "name = 'value'");
                }
                #[test]
                fn like() {
                    assert_eq!(Like::column("name").to_string(), "name LIKE ???");
                    assert_eq!(Like::column("name").value("'*value*'").to_string(), "name LIKE '%value%'");
                }
                #[test]
                fn between() {
                    assert_eq!(Between::column("name").to_string(), "name BETWEEN ??? AND ???");
                    assert_eq!(Between::column("name").from("1").to_string(), "name BETWEEN 1 AND ???");
                    assert_eq!(Between::column("name").thru("'thru'").to_string(), "name BETWEEN ??? AND 'thru'");
                    assert_eq!(Between::column("name").from("1").thru("10").to_string(), "name BETWEEN 1 AND 10");
                }
                #[test]
                fn or() {
                    assert_eq!(Or::conditions([]).to_string(), "");
                    assert_eq!(Or::conditions([equals!("name", "value")]).to_string(), "(name = value)");
                    assert_eq!(
                        Or::conditions([equals!("name", "value1"), equals!("name", "value2")]).to_string(),
                        "(name = value1 OR name = value2)"
                    );
                }
                #[test]
                fn conditions() {
                    let condition = or!([
                        or!([equals!("name", "value"), equals!("name", "value2")]),
                        like!("name", "*v"),
                        between!("other", "1", "5"),
                    ]);
                    assert_eq!(
                        condition.to_string(),
                        "((name = value OR name = value2) OR name LIKE '%v' OR other BETWEEN 1 AND 5)"
                    );
                }
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use entities::DbConfig;

            fn testenv(fixture: &testlib::TestFixture) -> Connection {
                let test_files = testlib::test_resources().join("db");
                fixture.copy_resources(&test_files);
                let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
                admin::init_db(&weather_dir, DbConfig::hybrid(), true, true, 1).unwrap();
                db_conn!(&weather_dir).unwrap()
            }

            #[test]
            fn query_locations() {
                let fixture = testlib::TestFixture::create();
                let conn = testenv(&fixture);
                let locations = get(&conn, &vec![], true).unwrap();
                assert_eq!(locations.len(), 3);
                for (location, expected_alias) in locations.iter().zip(["between", "north", "south"].iter()) {
                    assert_eq!(location.alias, *expected_alias);
                }
                let locations = get(&conn, &vec!["south".to_string(), "north".to_string()], true).unwrap();
                assert_eq!(locations.len(), 2);
                for (location, expected_alias) in locations.iter().zip(["north", "south"].iter()) {
                    assert_eq!(location.alias, *expected_alias);
                }
            }
            #[test]
            fn locations_sql() {
                macro_rules! normalize {
                    ($sql:expr) => {
                        $sql.split_whitespace().collect::<Vec<&str>>().join(" ")
                    };
                }
                assert_eq!(
                    normalize!(query_sql(&vec![], false)),
                    "SELECT name, alias, longitude, latitude, tz FROM locations"
                );
                assert_eq!(
                    normalize!(query_sql(&vec!["*".to_string()], true)),
                    "SELECT name, alias, longitude, latitude, tz FROM locations ORDER BY name ASC"
                );
                assert_eq!(
                    normalize!(query_sql(&vec!["*ern*".to_string()], true)),
                    "SELECT name, alias, longitude, latitude, tz FROM locations WHERE (name LIKE '%ern%' OR alias LIKE '%ern%') ORDER BY name ASC"
                );
                assert_eq!(
                    normalize!(query_sql(&vec!["*ern*".to_string(), "bet*".to_string()], false)),
                    "SELECT name, alias, longitude, latitude, tz FROM locations WHERE ((name LIKE '%ern%' OR alias LIKE '%ern%') OR (name LIKE 'bet%' OR alias LIKE 'bet%'))"
                );
                assert_eq!(
                    normalize!(query_sql(&vec!["*ern*".to_string(), "*".to_string()], false)),
                    "SELECT name, alias, longitude, latitude, tz FROM locations"
                );
            }
        }
    }

    /// The metadata insert SQL used by the [DataAdapter] implementations.
    const METADATA_SQL: &str = r#"
    INSERT INTO metadata (lid, date, store_size, size, mtime)
        VALUES (:lid, :date, :store_size, :size, :mtime)
    "#;

    mod hybrid {
        //! The database hybrid implementation
        use super::*;
        use crate::{
            backend::{filesys::WeatherHistory, DataAdapter, Result},
            prelude::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location},
        };
        use rusqlite::named_params;

        /// Create the *hybrid* version of the data adapter.
        pub(crate) fn data_adapter(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
            Ok(Box::new(HybridDataAdapter(weather_dir)))
        }

        /// The *hybrid* data adapter implementation.
        #[derive(Debug)]
        pub(crate) struct HybridDataAdapter(
            /// The weather data directory.
            WeatherDir,
        );
        impl DataAdapter for HybridDataAdapter {
            /// Returns the daily weather data history for a location.
            ///
            /// Currently it is an error if more than 1 location is found through the location
            /// query.
            ///
            /// # Arguments
            ///
            /// * `location` is whose historical data will be found.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, location: Location, history_range: DateRange) -> Result<DailyHistories> {
                let file = self.0.archive(&location.alias);
                let archive = WeatherHistory::new(&location.alias, file)?;
                let daily_histories = archive.daily_histories(&history_range)?;
                Ok(DailyHistories { location, histories: daily_histories })
            }
            /// Get the weather history dates for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations.
            fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
                let conn = db_conn!(&self.0)?;
                query::history_dates(&conn, criteria)
            }
            /// Get a summary of the weather history available for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations that should be used.
            fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
                let conn = db_conn!(&self.0)?;
                let mut history_summaries = query::history_summaries(&conn, criteria)?;
                // scan the archives to get the overall size
                for history_summary in &mut history_summaries {
                    let file = self.0.archive(&history_summary.location.alias);
                    history_summary.overall_size.replace(file.size() as usize);
                }
                Ok(history_summaries)
            }
            /// Get the metadata for weather locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations of interest.
            fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
                let conn = db_conn!(&self.0)?;
                locations::get(&conn, &criteria.filters, criteria.sort)
            }
        }

        /// Loads the database based on the *hybrid* implementation of weather data.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `weather_dir` is the weather data directory.
        pub(super) fn load(conn: &mut Connection, weather_dir: &WeatherDir) -> Result<()> {
            log::debug!("  metadata");
            let id_aliases = locations::id_aliases(conn)?;
            let tx = conn.transaction()?;
            // scope the statment to this block
            {
                let mut stmt = tx.prepare(METADATA_SQL)?;
                for (lid, alias) in id_aliases {
                    log::debug!("    {}", alias);
                    let file = weather_dir.archive(&alias);
                    let archive = WeatherArchive::open(&alias, file)?;
                    for md in archive.archive_iter(None, false, ArchiveMd::new)? {
                        stmt.execute(named_params! {
                            ":lid": lid,
                            ":date": &md.date,
                            ":store_size": md.compressed_size,
                            ":size": md.size,
                            ":mtime": md.mtime
                        })?;
                    }
                }
            }
            tx.commit()?;
            Ok(())
        }
    }

    mod document {
        //! The database document implementation
        use super::*;
        use crate::{
            backend::{filesys::ArchiveData, DataAdapter, Error, Result},
            prelude::{DailyHistories, DataCriteria, DateRange, History, HistoryDates, HistorySummaries, Location},
        };
        use chrono::NaiveDate;
        use rusqlite::{blob::ZeroBlob, named_params, Transaction};
        use serde_json::Value;

        /// Create the *document* version of the data adapter.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory name.
        pub(crate) fn data_adapter(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
            let data_adapter = DocumentDataAdapter(weather_dir);
            Ok(Box::new(data_adapter))
        }

        /// The *document* data adapter implementation.
        #[derive(Debug)]
        pub(crate) struct DocumentDataAdapter(
            /// The weather data directory.
            WeatherDir,
        );
        impl DataAdapter for DocumentDataAdapter {
            /// Returns the daily weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `location` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories> {
                let conn = db_conn!(&self.0)?;
                let daily_histories = query_daily_history(&conn, &location.alias, date_range)?;
                Ok(DailyHistories { location, histories: daily_histories })
            }
            /// Get the weather history dates for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations.
            fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
                let conn = db_conn!(&self.0)?;
                query::history_dates(&conn, criteria)
            }
            /// Get a summary of the weather history available for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations that should be used.
            fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
                let conn = db_conn!(&self.0)?;
                let mut history_summaries = query::history_summaries(&conn, criteria)?;
                // this is terribly expensize but it fullfills the contract and is still cheaper than file archives
                for (alias, overall_size) in query::table_size(&conn, "documents")? {
                    for history_summary in &mut history_summaries {
                        if history_summary.location.alias == alias {
                            history_summary.overall_size = Some(overall_size);
                            break;
                        }
                    }
                }
                Ok(history_summaries)
            }
            /// Get the metadata for weather locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations of interest.
            fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
                let conn = db_conn!(&self.0)?;
                locations::get(&conn, &criteria.filters, criteria.sort)
            }
        }

        /// The `SQL` select statement used by the *document* implementation.
        const SELECT_SQL: &str = r#"
            SELECT
                l.id AS lid,
                m.date AS date,
                d.id as document_id,
                d.daily AS daily,
                d.daily_size AS size
            FROM locations AS l
                INNER JOIN metadata AS m ON l.id=m.lid
                INNER JOIN documents AS d ON m.id=d.mid
            WHERE
                l.alias=:alias AND m.date BETWEEN :from AND :thru
            ORDER BY date"#;

        /// Query the database for daily history.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `alias` is the location alias.
        /// * `date_range` identifies what daily history will be returned.
        pub fn query_daily_history(conn: &Connection, alias: &str, date_range: DateRange) -> Result<Vec<History>> {
            let db_config = admin::database_configuration(&conn)?;
            let mut stmt = conn.prepare(SELECT_SQL)?;
            let mut rows =
                stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
            let mut daily_histories = vec![];
            while let Some(row) = rows.next()? {
                let json_text: String = if db_config.compress {
                    let rid: i64 = row.get("document_id")?;
                    let data = blob::read(conn, "documents", "daily_zip", rid)?;
                    let json = compression::uncompress_str(&data[..])?;
                    json
                } else {
                    row.get("daily")?
                };
                let history = history::from_bytes(alias, json_text.as_bytes())?;
                daily_histories.push(history);
            }
            Ok(daily_histories)
        }

        pub(crate) use darksky::load;
        mod darksky {
            //! The document database archive loader for DarkSky weather data.
            use super::*;
            use archive_loader::*;
            use rusqlite::types::Null;
            use std::{
                sync::mpsc::{self, TryRecvError},
                thread, time,
            };

            /// The data passed through the [ArchiveLoader].
            #[derive(Debug)]
            struct LoadMsg {
                /// The location table identifier.
                pub lid: i64,
                /// The history date
                pub date: NaiveDate,
                /// The size of the daily history
                pub size: usize,
                /// The store size of the daily history.
                pub store_size: usize,
                /// Indicate if the history is compressed or not
                pub compressed: bool,
                /// The daily history.
                pub history: Vec<u8>,
            }

            /// The type definition for the document producer.
            type Sender = mpsc::Sender<LoadMsg>;

            /// The type definition for the document consummer.
            type Receiver = mpsc::Receiver<LoadMsg>;

            /// Take the DarkSky archives and push them into the database.
            ///
            /// # Argument
            ///
            /// * `weather_dir` is the weather data directory.
            /// * `threads` is the number of workers to use getting data from archives.
            pub(crate) fn load(weather_dir: &WeatherDir, compress: bool, threads: usize) -> Result<()> {
                let conn = db_conn!(weather_dir)?;
                let archives = ArchiveQueue::new(&conn, weather_dir)?;
                let mut loader: ArchiveLoader<LoadMsg> = ArchiveLoader::new(threads);
                loader.execute(archives, || Box::new(DarkskyProducer(compress)), || Box::new(DocumentConsummer(conn)))
            }

            struct DarkskyProducer(
                /// Indicates the history document should be compressed
                bool,
            );
            impl DarkskyProducer {
                /// This is called to semd the archive data to the [ArchiveConsummer].
                ///
                /// # Arguments
                ///
                /// * `lid` is the locations primary id in the database.
                /// * `alias` is the locations alias name.
                /// * `date` is the date associated with the archive data.
                /// * `json` is archive data as parsed `JSON`.
                /// * `sender` is used to pass data to the collector.
                #[rustfmt::skip]
                fn send_history(&self, lid: i64, alias: &str, date: NaiveDate, json: Value, sender: &Sender) -> Result<()> {
                    match serde_json::to_string(&json) {
                        Ok(history) => {
                            let size = history.len();
                            let history = match self.0 {
                                true => compression::compress_str(&history)?,
                                false => history.into_bytes()
                            };
                            let store_size = history.len();
                            let msg = LoadMsg { lid, date, size, store_size, compressed: self.0, history };
                            match sender.send(msg) {
                                Ok(_) => Ok(()),
                                Err(_) => Err(Error::from("Document SendError...")),
                            }
                        }
                        Err(err) => {
                            let reason = format!("Error JSON to_string for {} {} ({}).", alias, date, &err);
                            Err(Error::from(reason))
                        }
                    }
                }
            }
            impl ArchiveProducer<LoadMsg> for DarkskyProducer {
                /// This is called by the [ArchiveProducer] to pull data from the archive.
                ///
                /// # Arguments
                ///
                /// * `lid` is the locations primary id in the database.
                /// * `alias` is the locations alias name.
                /// * `file` is the weather data archive.
                /// * `sender` is used to pass data to the collector.
                fn gather(&self, lid: i64, alias: &str, file: WeatherFile, sender: &Sender) -> Result<usize> {
                    let archive = WeatherArchive::open(&alias, file)?;
                    let mut result = Ok(0);
                    for data in archive.archive_iter(None, false, ArchiveData::new)? {
                        let json = data.json()?["daily"]["data"][0].take();
                        if json.is_object() {
                            self.send_history(lid, alias, data.date, json, sender)?;
                            match result.as_mut() {
                                Ok(count) => *count += 1,
                                Err(_) => unreachable!("Result is an error and shouldn't be..."),
                            };
                        } else {
                            let reason = format!("Error getting history for {} on {} (not daily).", alias, data.date);
                            result = Err(Error::from(reason));
                            break;
                        }
                    }
                    result
                }
            }

            /// The `SQL` used to *insert* history.
            const INSERT_SQL: &str = r#"
            INSERT INTO documents (mid, daily, daily_zip, daily_size)
                VALUES (:mid, :daily, :daily_zip, :daily_size)
            "#;

            /// The database history loader.
            struct DocumentConsummer(
                /// The database connection that will be used.
                Connection,
            );
            impl DocumentConsummer {
                /// Add weather history to the database.
                ///
                /// This is a `static` method in order to separate collection from adding data. It can't be
                /// an instance method because it would require borrowing mutable from an instance already
                /// mutable.
                ///
                /// # Arguments
                ///
                /// * `tx` is the transaction associate with the data insertion.
                /// * `msg` contains the history that will be added to the database.
                fn insert_history(tx: &mut Transaction, msg: LoadMsg) -> Result<()> {
                    let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
                    let mut md_stmt = tx.prepare_cached(METADATA_SQL)?;
                    md_stmt.execute(named_params! {
                        ":lid": msg.lid,
                        ":date": msg.date,
                        ":store_size": msg.store_size,
                        ":size": msg.size,
                        ":mtime": 0
                    })?;
                    let mid = tx.last_insert_rowid();
                    if msg.compressed {
                        data_stmt.execute(named_params! {
                            ":mid": mid,
                            ":daily": Null,
                            ":daily_zip": ZeroBlob(msg.store_size as i32),
                            ":daily_size": msg.size
                        })?;
                        let rid = tx.last_insert_rowid();
                        blob::write(tx, &msg.history, "documents", "daily_zip", rid)?;
                    } else {
                        data_stmt.execute(named_params! {
                            ":mid": mid,
                            ":daily": msg.history,
                            ":daily_zip": Null,
                            ":daily_size": msg.size
                        })?;
                    }
                    Ok(())
                }
            }
            impl ArchiveConsummer<LoadMsg> for DocumentConsummer {
                /// Called by the [ArchiveLoader] to collect the weather history being mined.
                ///
                /// # Arguments
                ///
                /// * `receiver` is used to collect the weather data.
                fn collect(&mut self, receiver: Receiver) -> Result<usize> {
                    let mut tx = self.0.transaction()?;
                    let mut count: usize = 0;
                    // spin on the receiver until there's no one sending more data
                    let pause = time::Duration::from_millis(1);
                    loop {
                        match receiver.try_recv() {
                            Ok(msg) => {
                                Self::insert_history(&mut tx, msg)?;
                                count += 1;
                            }
                            Err(err) => match err {
                                TryRecvError::Empty => thread::sleep(pause),
                                TryRecvError::Disconnected => break,
                            },
                        }
                    }
                    // commit the load
                    match tx.commit() {
                        Ok(_) => Ok(count),
                        Err(err) => {
                            let reason = format!("Error commiting load transaction ({}).", &err);
                            Err(Error::from(reason))
                        }
                    }
                }
            }
        }

        mod compression {
            //! Isolate the `Snappy` compression format here
            use super::{Error, Result};
            use snap::{read, write};
            use std::io::{Read, Write};

            /// Compress a string using `snap`.
            ///
            /// # Arguments
            ///
            /// * `data` is the string that will be compressed.
            pub(super) fn compress_str(data: &str) -> Result<Vec<u8>> {
                compress(data.as_bytes())
            }

            /// Compress a sequence of bytes.
            ///
            /// # Argument
            ///
            /// * `data` is the sequence of bytes that will be compressed.
            fn compress(data: &[u8]) -> Result<Vec<u8>> {
                let mut writer = write::FrameEncoder::new(vec![]);
                match writer.write_all(data) {
                    Ok(_) => match writer.into_inner() {
                        Ok(compressed_data) => Ok(compressed_data),
                        Err(err) => {
                            let reason = format!("Error getting compressed data ({})", err);
                            Err(Error::from(reason))
                        }
                    },
                    Err(err) => {
                        let reason = format!("Error compressing data ({})", err);
                        Err(Error::from(reason))
                    }
                }
            }

            /// Uncompress a sequence of bytes into a string using `snap`.
            ///
            /// # Arguments
            ///
            /// * `compressed_data` is what will be uncompressed and converted to a string.
            pub(super) fn uncompress_str(compressed_data: &[u8]) -> Result<String> {
                match String::from_utf8(uncompress(compressed_data)?) {
                    Ok(string) => Ok(string),
                    Err(err) => {
                        let reason = format!("Error reading UTF8 ({})", err);
                        Err(Error::from(reason))
                    }
                }
            }

            /// Uncompress a sequence of bytes into a sequence of uncompressed bytes.
            ///
            /// # Arguments
            ///
            /// * `compressed_data` is what will be uncompressed into a sequence of bytes.
            pub(super) fn uncompress(compressed_data: &[u8]) -> Result<Vec<u8>> {
                let mut data = vec![];
                match read::FrameDecoder::new(&compressed_data[..]).read_to_end(&mut data) {
                    Ok(_) => Ok(data),
                    Err(err) => {
                        let reason = format!("Error reading compressed data ({})", err);
                        Err(Error::from(reason))
                    }
                }
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn compress_uncompress() {
                    let testcase = include_str!("db/schema.sql");
                    let compressed_data = compress_str(testcase).unwrap();
                    assert_ne!(compressed_data.len(), testcase.len());
                    let uncompressed_data = uncompress_str(&compressed_data[..]).unwrap();
                    assert_eq!(testcase, uncompressed_data)
                }
            }
        }

        mod blob {
            //! This isolates what it takes to read and write blobs in the database.
            use super::*;
            use std::io::{Read, Write};

            /// Writes a *blob* into the database. This is specific to `sqlite3`.
            ///
            /// # Arguments
            ///
            /// * `tx` is the transaction used to write to the database.
            /// * `table` is the table that will hold the *blob*.
            /// * `column` is the database column defined as a *blob*.
            /// * `rid` is the row identifier of the *blob*.
            pub(super) fn write(tx: &Transaction, history: &[u8], table: &str, column: &str, rid: i64) -> Result<()> {
                match tx.blob_open(rusqlite::DatabaseName::Main, table, column, rid, false) {
                    Ok(mut blob) => match blob.write_all(history) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            let reason = format!("Error writing blob {}({}) ({})", table, column, err);
                            Err(Error::from(reason))
                        }
                    },
                    Err(err) => {
                        let reason = format!("Error opening blob writer on {}({}) ({})", table, column, err);
                        Err(Error::from(reason))
                    }
                }
            }

            /// Read a *blob* from the database. This is specific to `sqlite3`.
            ///
            /// # Arguments
            ///
            /// * `conn` is the database connection that will be used.
            /// * `table` is the table that will hold the *blob*.
            /// * `column` is the database column defined as a *blob*.
            /// * `rid` is the row identifier of the *blob*.
            pub(super) fn read(conn: &Connection, table: &str, column: &str, rid: i64) -> Result<Vec<u8>> {
                match conn.blob_open(rusqlite::DatabaseName::Main, table, column, rid, true) {
                    Ok(mut blob) => {
                        let mut compressed_data: Vec<u8> = vec![0; blob.len()];
                        match blob.read_exact(&mut compressed_data[..]) {
                            Ok(_) => Ok(compressed_data),
                            Err(err) => {
                                let reason = format!("Error reading blob {}({}) ({})", table, column, err);
                                Err(Error::from(reason))
                            }
                        }
                    }
                    Err(err) => {
                        let reason = format!("Error opening blob reader on {}({}) ({})", table, column, err);
                        Err(Error::from(reason))
                    }
                }
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn blob() {
                    // house cleaning
                    // let db_name = "temp.db";
                    // if std::path::PathBuf::from(db_name).exists() {
                    //     std::fs::remove_file(db_name).unwrap();
                    // }
                    // let conn = db_connection(Some("temp.db")).unwrap();
                    let mut conn = db_connection(None).unwrap();
                    // create a test db
                    let schema = r#"
                CREATE TABLE example
                (
                    id INTEGER PRIMARY KEY,
                    mid INTEGER NOT NULL,
                    data BLOB
                );"#;
                    conn.execute_batch(schema).unwrap();
                    // compress up some data
                    let testcase = include_str!("db/schema.sql");
                    let compressed = compression::compress_str(testcase).unwrap();
                    // now insert the data
                    let size = compressed.len();
                    let row_id;
                    {
                        let tx = conn.transaction().unwrap();
                        let insert = "INSERT INTO example (mid, data) VALUES (?1, ?2)";
                        tx.execute(insert, (1, ZeroBlob(size as i32))).unwrap();
                        row_id = tx.last_insert_rowid();
                        write(&tx, &compressed[..], "example", "data", row_id).unwrap();
                        tx.commit().unwrap();
                    }
                    let compressed_data = read(&conn, "example", "data", row_id).unwrap();
                    assert_eq!(compressed, compressed_data);
                    let uncompressed_data = compression::uncompress_str(&compressed_data[..]).unwrap();
                    assert_eq!(testcase, uncompressed_data);
                }
            }
        }
    }

    mod normalized {
        //! The [DataAdapter] implementation using a normalized datbase schema.
        use super::*;
        use backend::{
            filesys::{ArchiveData, WeatherHistoryUpdate},
            DataAdapter, Error, Result,
        };
        use entities::{DailyHistories, DataCriteria, DateRange, History, HistoryDates, HistorySummaries, Location};
        use rusqlite::{named_params, Transaction};

        /// Create the *normalized* version of the data adapter.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory name.
        pub(crate) fn data_adapter(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
            let data_adapter = NormalizedDataAdapter(weather_dir);
            Ok(Box::new(data_adapter))
        }

        /// The *normalized* data adapter implementation.
        pub(crate) struct NormalizedDataAdapter(
            /// The weather data directory.
            WeatherDir,
        );
        impl DataAdapter for NormalizedDataAdapter {
            /// Returns the daily weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `location` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories> {
                let conn = db_conn!(&self.0)?;
                let daily_histories = query_history(&conn, &location.alias, date_range)?;
                Ok(DailyHistories { location, histories: daily_histories })
            }
            /// Get the weather history dates for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations.
            fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
                let conn = db_conn!(&self.0)?;
                query::history_dates(&conn, criteria)
            }
            /// Get a summary of the weather history available for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations that should be used.
            fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
                // get the archives history summary
                let archive_summary =
                    backend::filesys::archive_adapter(self.0.to_string().as_str())?.history_summaries(
                        DataCriteria { filters: criteria.filters.clone(), icase: criteria.icase, sort: criteria.sort },
                    )?;
                let conn = db_conn!(&self.0)?;
                let db_summaries = query::table_size(&conn, "history")?;
                let history_summaries = archive_summary
                    .into_iter()
                    .map(|hs| {
                        let db_size =
                            db_summaries
                                .iter()
                                .find_map(|(alias, size)| if alias == &hs.location.alias { Some(*size) } else { None });
                        let overall_size = if hs.overall_size.is_none() && db_size.is_none() {
                            None
                        } else {
                            let archive_size = hs.overall_size.map_or(0, |s| s);
                            let db_size = db_size.map_or(0, |s| s);
                            Some(archive_size + db_size)
                        };
                        HistorySummaries {
                            location: hs.location,
                            count: hs.count,
                            overall_size,
                            raw_size: db_size,
                            store_size: hs.overall_size,
                        }
                    })
                    .collect();
                Ok(history_summaries)
            }
            /// Get the metadata for weather locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations of interest.
            fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
                let conn = db_conn!(&self.0)?;
                locations::get(&conn, &criteria.filters, criteria.sort)
            }
            /// Add weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies what location should be used.
            /// * `date_range` specifies the date range that should be used.
            fn add_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
                // add histories to the archive first
                let location = &daily_histories.location;
                let file = self.0.archive(&location.alias);
                let mut history_updater = WeatherHistoryUpdate::new(&location.alias, file)?;
                let archive_additions = history_updater.add(&daily_histories.histories)?;
                // filter out histories that already exist in the db
                let mut conn = db_conn!(&self.0)?;
                let lid = locations::location_id(&conn, &location.alias)?;
                let histories = query::histories_not_found(&conn, lid, &daily_histories.histories)?;
                let histories_len = histories.len();
                if archive_additions != histories_len {
                    log::warn!(
                        "There were {} histories added to archive, {} added to db.",
                        archive_additions,
                        histories_len
                    );
                }
                // now add the histories.
                let stopwatch = StopWatch::start_new();
                let size = size_estimate(&conn, "history")?;
                let mut tx = conn.transaction()?;
                for history in histories {
                    let size = size
                        + history.description.as_ref().map_or(0, |s| s.len())
                        + history.precipitation_type.as_ref().map_or(0, |s| s.len());
                    insert_history(&mut tx, lid, size, history)?;
                }
                tx.commit()?;
                log::debug!("add_histories: {}", stopwatch);
                Ok(std::cmp::max(archive_additions, histories_len))
            }
        }

        /// The `SQL` used to insert normalized data into the database.
        const INSERT_SQL: &str = r#"
            INSERT INTO history (
                mid, temp_high, temp_low, temp_mean, dew_point,
                humidity, sunrise_t, sunset_t, cloud_cover, moon_phase,
                uv_index, wind_speed, wind_gust, wind_dir, visibility,
                pressure, precip, precip_prob, precip_type, description
            )
            VALUES (
                :mid, :temp_high, :temp_low, :temp_mean, :dew_point,
                :humidity, :sunrise_t, :sunset_t, :cloud_cover, :moon_phase,
                :uv_index, :wind_speed, :wind_gust, :wind_dir, :visibility, 
                :pressure, :precip, :precip_prob, :precip_type, :description
            )"#;
        /// Add weather history to the database.
        ///
        /// This is a `static` method in order to separate collection from adding data. It can't be
        /// an instance method because it would require borrowing mutable from an instance already
        /// mutable.
        ///
        /// # Arguments
        ///
        /// * `tx` is the transaction associate with the data insertion.
        /// * `msg` contains the history that will be added to the database.
        fn insert_history(tx: &mut Transaction, lid: i64, size: usize, history: &History) -> Result<()> {
            let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
            let mut md_stmt = tx.prepare_cached(METADATA_SQL)?;
            md_stmt.execute(named_params! {
                ":lid": lid,
                ":date": &history.date,
                ":store_size": size,
                ":size": size,
                ":mtime": 0
            })?;
            let mid = tx.last_insert_rowid();
            data_stmt.execute(named_params! {
                ":mid": mid,
                ":temp_high": history.temperature_high,
                ":temp_low": history.temperature_low,
                ":temp_mean": history.temperature_mean,
                ":dew_point": history.dew_point,
                ":humidity": history.humidity,
                ":sunrise_t": history.sunrise,
                ":sunset_t": history.sunset,
                ":cloud_cover": history.cloud_cover,
                ":moon_phase": history.moon_phase,
                ":uv_index": history.uv_index,
                ":wind_speed": history.wind_speed,
                ":wind_gust": history.wind_gust,
                ":wind_dir": history.wind_direction,
                ":visibility": history.visibility,
                ":pressure": history.pressure,
                ":precip": history.precipitation_amount,
                ":precip_prob": history.precipitation_chance,
                ":precip_type": history.precipitation_type,
                ":description": history.description,
            })?;
            Ok(())
        }

        /// Loads the database based on the *normalized* implementation of weather data.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `weather_dir` is the weather data directory.
        /// * `threads` is the number of threads to use loading data.
        pub(super) fn load(weather_dir: &WeatherDir, threads: usize) -> Result<()> {
            loader::run(weather_dir, threads)
        }

        /// The `SQL` used to select history data from the database.
        const HISTORY_SQL: &str = r#"
        SELECT
            l.id AS lid, m.date AS date,
            h.temp_high AS temp_high, h.temp_low AS temp_low, h.temp_mean AS temp_mean,
            h.dew_point AS dew_point, h.humidity AS humidity,
            h.sunrise_t AS sunrise_t, h.sunset_t AS sunset_t, 
            h.cloud_cover AS cloud_cover, h.moon_phase AS moon_phase, h.uv_index AS uv_index,
            h.wind_speed AS wind_speed, h.wind_gust AS wind_gust, h.wind_dir AS wind_dir,
            h.visibility as visibility, h.pressure as pressure,
            h.precip as precip, h.precip_prob as precip_prob, h.precip_type as precip_type,
            h.description AS description
        FROM locations AS l
            INNER JOIN metadata AS m ON l.id=m.lid
            INNER JOIN history AS h ON m.id=h.mid
        WHERE
            l.alias=:alias AND m.date BETWEEN :from AND :thru
        ORDER BY date
        "#;

        /// Get history from the database.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `alias` is the location alias name.
        /// * `date_range` determines the daily history.
        fn query_history(conn: &Connection, alias: &str, date_range: DateRange) -> Result<Vec<History>> {
            let mut stmt = conn.prepare(HISTORY_SQL)?;
            let mut rows =
                stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
            let mut daily_histories = vec![];
            while let Some(row) = rows.next()? {
                let history = History {
                    alias: alias.to_string(),
                    date: row.get("date")?,
                    temperature_high: row.get("temp_high")?,
                    temperature_low: row.get("temp_low")?,
                    temperature_mean: row.get("temp_mean")?,
                    dew_point: row.get("dew_point")?,
                    humidity: row.get("humidity")?,
                    precipitation_chance: row.get("precip_prob")?,
                    precipitation_type: row.get("precip_type")?,
                    precipitation_amount: row.get("precip")?,
                    wind_speed: row.get("wind_speed")?,
                    wind_gust: row.get("wind_gust")?,
                    wind_direction: row.get("wind_dir")?,
                    cloud_cover: row.get("cloud_cover")?,
                    pressure: row.get("pressure")?,
                    uv_index: row.get("uv_index")?,
                    sunrise: row.get("sunrise_t")?,
                    sunset: row.get("sunset_t")?,
                    moon_phase: row.get("moon_phase")?,
                    visibility: row.get("visibility")?,
                    description: row.get("description")?,
                };
                daily_histories.push(history);
            }
            Ok(daily_histories)
        }

        /// Get the size estimate of a table in the database. This is specific to `sqlite`.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `table` is the database table name.
        fn size_estimate(conn: &Connection, table: &str) -> Result<usize> {
            // the primary id will always be 64 bit
            let mut size_estimate = 8;
            // this is specific to sqlite3
            conn.pragma(None, table, table, |row| {
                let name: String = row.get("name")?;
                let column_type: String = row.get("type")?;
                match column_type.as_str() {
                    "REAL" => size_estimate += 8,
                    "INTEGER" => {
                        if name.ends_with("_t") {
                            size_estimate += 8;
                        } else {
                            size_estimate += 4;
                        }
                    }
                    "TEXT" => (),
                    _ => {
                        eprintln!("Yikes!!!! Did not recognize column {} type '{}'...", name, column_type);
                    }
                }
                Ok(())
            })?;
            Ok(size_estimate)
        }

        mod loader {
            //! The normalized database archive loader for [History].
            use super::*;
            use archive_loader::*;
            use std::{
                sync::mpsc::{Receiver, Sender, TryRecvError},
                thread, time,
            };

            /// The data passed through the [ArchiveLoader].
            #[derive(Debug)]
            struct LoadMsg {
                /// The location table identifier.
                pub lid: i64,
                /// The size of the daily history
                pub size: usize,
                // /// The store size of the daily history.
                // pub store_size: usize,
                /// The daily history.
                pub history: History,
            }

            /// Take the [History] archives and push them into the database.
            ///
            /// # Argument
            ///
            /// * `weather_dir` is the weather data directory.
            /// * `threads` is the number of workers to use getting data from archives.
            pub(crate) fn run(weather_dir: &WeatherDir, threads: usize) -> Result<()> {
                let conn = db_conn!(weather_dir)?;
                // get an estimate of the raw size of data, 32 is overhead for the text
                let size_estimate = size_estimate(&conn, "history")?;
                let archives = ArchiveQueue::new(&conn, weather_dir)?;
                let mut loader: ArchiveLoader<LoadMsg> = ArchiveLoader::new(threads);
                loader.execute(
                    archives,
                    || Box::new(HistoryProducer(size_estimate)),
                    || Box::new(HistoryConsummer(conn)),
                )
            }

            /// The [History] data producer.
            struct HistoryProducer(
                /// The estimated size of data within the database.
                usize,
            );
            impl HistoryProducer {
                /// Send the history data to the consummer side of the loader.
                ///
                /// # Arguments
                ///
                /// * `lid` is the locations primary id in the database.
                /// * `history` is the data that will be sent off to the consummer.
                /// * `sender` is used to pass data to the collector.
                fn send_history(&self, lid: i64, history: History, sender: &Sender<LoadMsg>) -> Result<()> {
                    let mut size = self.0 + history.description.as_ref().map_or(0, |s| s.len());
                    size += history.precipitation_type.as_ref().map_or(Default::default(), |t| t.len());
                    let msg = LoadMsg { lid, size, history };
                    match sender.send(msg) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(Error::from("SendError...")),
                    }
                }
            }
            impl ArchiveProducer<LoadMsg> for HistoryProducer {
                /// This is called by the archive producer to get data from the archive.
                ///
                /// # Arguments
                ///
                /// * `lid` is the locations primary id in the database.
                /// * `alias` is the locations alias name.
                /// * `file` is the weather data archive.
                /// * `sender` is used to pass data to the collector.
                fn gather(&self, lid: i64, alias: &str, file: WeatherFile, sender: &Sender<LoadMsg>) -> Result<usize> {
                    let archive = WeatherArchive::open(&alias, file)?;
                    let mut result = Ok(0);
                    for data in archive.archive_iter(None, false, ArchiveData::new)? {
                        // let json = data.json()?;
                        match backend::history::from_bytes(&alias, data.bytes()) {
                            Ok(history) => {
                                self.send_history(lid, history, sender)?;
                                match result.as_mut() {
                                    Ok(count) => *count += 1,
                                    Err(_) => unreachable!("Result is an error and shouldn't be..."),
                                };
                            }
                            Err(err) => {
                                result = Err(err);
                                break;
                            }
                        }
                    }
                    result
                }
            }

            /// The database history loader.
            struct HistoryConsummer(
                /// The database connection that will be used.
                Connection,
            );
            impl HistoryConsummer {
                // /// Add weather history to the database.
                // ///
                // /// This is a `static` method in order to separate collection from adding data. It can't be
                // /// an instance method because it would require borrowing mutable from an instance already
                // /// mutable.
                // ///
                // /// # Arguments
                // ///
                // /// * `tx` is the transaction associate with the data insertion.
                // /// * `msg` contains the history that will be added to the database.
                // fn insert_history(tx: &mut Transaction, msg: LoadMsg) -> Result<()> {
                //     let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
                //     let mut md_stmt = tx.prepare_cached(METADATA_SQL)?;
                //     md_stmt.execute(named_params! {
                //         ":lid": msg.lid,
                //         ":date": &msg.history.date,
                //         ":store_size": msg.store_size,
                //         ":size": msg.size,
                //         ":mtime": 0
                //     })?;
                //     let mid = tx.last_insert_rowid();
                //     data_stmt.execute(named_params! {
                //         ":mid": mid,
                //         ":temp_high": msg.history.temperature_high,
                //         ":temp_low": msg.history.temperature_low,
                //         ":temp_mean": msg.history.temperature_mean,
                //         ":dew_point": msg.history.dew_point,
                //         ":humidity": msg.history.humidity,
                //         ":sunrise_t": msg.history.sunrise,
                //         ":sunset_t": msg.history.sunset,
                //         ":cloud_cover": msg.history.cloud_cover,
                //         ":moon_phase": msg.history.moon_phase,
                //         ":uv_index": msg.history.uv_index,
                //         ":wind_speed": msg.history.wind_speed,
                //         ":wind_gust": msg.history.wind_gust,
                //         ":wind_dir": msg.history.wind_direction,
                //         ":visibility": msg.history.visibility,
                //         ":pressure": msg.history.pressure,
                //         ":precip": msg.history.precipitation_amount,
                //         ":precip_prob": msg.history.precipitation_chance,
                //         ":precip_type": msg.history.precipitation_type,
                //         ":description": msg.history.description,
                //     })?;
                //     Ok(())
                // }
            }
            impl ArchiveConsummer<LoadMsg> for HistoryConsummer {
                /// /// Called by the [ArchiveLoader] to collect the weather history being mined.
                ///
                /// # Arguments
                ///
                /// * `receiver` is used to collect the weather data.
                fn collect(&mut self, receiver: Receiver<LoadMsg>) -> Result<usize> {
                    let mut tx = self.0.transaction()?;
                    let mut count: usize = 0;
                    // spin on the receiver until there's no one sending more data
                    let pause = time::Duration::from_millis(1);
                    loop {
                        match receiver.try_recv() {
                            Ok(msg) => {
                                super::insert_history(&mut tx, msg.lid, msg.size, &msg.history)?;
                                count += 1;
                            }
                            Err(err) => match err {
                                TryRecvError::Empty => thread::sleep(pause),
                                TryRecvError::Disconnected => break,
                            },
                        }
                    }
                    // commit the load
                    match tx.commit() {
                        Ok(_) => Ok(count),
                        Err(err) => {
                            let reason = format!("Error commiting load transaction ({}).", &err);
                            Err(Error::from(reason))
                        }
                    }
                }
            }
        }
    }

    pub(super) mod archive_loader {
        //! A threaded history data loader.
        // use crate::backend::db::v1::document::load;

        use super::*;
        use std::{
            marker::PhantomData,
            sync::{mpsc, Arc, Mutex},
            thread,
        };
        use toolslib::{fmt::commafy, stopwatch::StopWatch};

        /// A helper to log elapsed load times.
        macro_rules! log_elapsed {
            ($what:expr, $count:expr, $stopwatch:expr) => {{
                let per_msec = $count as f64 / $stopwatch.millis() as f64;
                log::debug!(
                    "{:?} {}: {} loaded in {} ({:0.3}history/ms).",
                    thread::current().id(),
                    $what,
                    commafy($count),
                    $stopwatch,
                    per_msec
                );
            }};
        }

        /// The trait used by the [ArchiveLoader] to gather data from a weather archive.
        pub(super) trait ArchiveProducer<T> {
            /// The *producer* side of the archive data.
            ///
            /// # Arguments
            ///
            /// * `sender` is used to hand off the gathered archive data.
            /// * `archives` is a collection of archives to gather data from.
            fn gather(&self, lid: i64, alias: &str, file: WeatherFile, sender: &mpsc::Sender<T>) -> Result<usize>;
            /// Trait boiler plate that gets archive metadata from the queue and calls the data extractor.
            fn send(&self, sender: mpsc::Sender<T>, archives: Arc<ArchiveQueue>) {
                while let Some(md) = archives.next() {
                    let mut load_time = StopWatch::start_new();
                    let filename = md.file.filename.clone();
                    match self.gather(md.lid, &md.alias, md.file, &sender) {
                        Ok(count) => {
                            load_time.stop();
                            self.log_elapsed(&md.alias, count, &load_time);
                        }
                        Err(err) => {
                            log::error!("{:?} error loading archive {} ({}).", thread::current().id(), filename, &err);
                            break;
                        }
                    }
                }
            }
            /// Trait boiler plate that logs elapsed time for the producer.
            ///
            /// # Arguments
            ///
            /// * `description` tersely describes the elapsed time.
            /// * `count` is the number of items mined from the archive.
            /// * `load_time` is how long the gather took.
            fn log_elapsed(&self, description: &str, count: usize, load_time: &StopWatch) {
                log_elapsed!(description, count, load_time);
            }
        }

        /// The trait used by the [ArchiveLoader] to collect the data gathered from weather archives.
        pub(super) trait ArchiveConsummer<T> {
            /// The *consummer* side of the archive data.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the gathered archive data.
            fn collect(&mut self, receiver: mpsc::Receiver<T>) -> Result<usize>;
            /// The boiler plate side for the *consummer* of archive data.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the gathered archive data.
            fn receive(&mut self, receiver: mpsc::Receiver<T>) {
                let mut load_time = StopWatch::start_new();
                match self.collect(receiver) {
                    Ok(count) => {
                        load_time.stop();
                        self.log_elapsed("Overall", count, &load_time);
                    }
                    Err(err) => {
                        let reason = format!("ArchiveConsummer collect error ({})", &err);
                        log::error!("{}", reason);
                    }
                }
            }
            /// Trait boiler plate that logs elapsed time for the consummer.
            ///
            /// # Arguments
            ///
            /// * `description` tersely describes the elapsed time.
            /// * `count` is the number of items mined from the archive.
            /// * `load_time` is how long the collection took.
            fn log_elapsed(&self, description: &str, count: usize, load_time: &StopWatch) {
                log_elapsed!(description, count, load_time);
            }
        }

        /// A threaded framework that gathers data from archives.
        #[derive(Debug)]
        pub(super) struct ArchiveLoader<T> {
            /// The number of threads to use.
            threads: usize,
            /// The **'I need to be associated with a type`** compiler hack.
            phantom: PhantomData<T>,
        }
        impl<T: 'static + Send> ArchiveLoader<T> {
            /// Create a new instance of the loader.
            ///
            /// # Arguments
            ///
            /// * `threads` is the number of threads to use gathering data.
            pub(super) fn new(threads: usize) -> ArchiveLoader<T> {
                Self { threads, phantom: PhantomData }
            }
            /// Gather data from a collection of archives.
            ///
            /// # Arguments
            ///
            /// * `archives` is the collection of archives data will be gathered from.
            /// * `producer` is used to create the threads that gather archive data.
            /// * `consummer` is used to create the collector of archive data.
            pub(super) fn execute<P, C>(&mut self, archives: ArchiveQueue, producer: P, consummer: C) -> Result<()>
            where
                P: Fn() -> Box<dyn ArchiveProducer<T> + Send>,
                C: FnOnce() -> Box<dyn ArchiveConsummer<T> + Send>,
            {
                // start up the threads that gather data
                let archives = Arc::new(archives);
                let (sender, receiver) = mpsc::channel::<T>();
                let mut handles = Vec::with_capacity(self.threads);
                for _ in 0..self.threads {
                    let producer = producer();
                    let sender = sender.clone();
                    let archive_queue = archives.clone();
                    let handle = thread::spawn(move || {
                        producer.send(sender, archive_queue);
                    });
                    handles.push(handle);
                }
                // now that the threads are running close down the sender
                drop(sender);
                // run the consummer
                consummer().receive(receiver);
                // now cleanup the threads
                for handle in handles {
                    let thread_id = handle.thread().id();
                    match handle.join() {
                        Ok(_) => (),
                        Err(_) => {
                            log::error!("Error joining with thread ({:?})", thread_id);
                        }
                    }
                }
                Ok(())
            }
        }

        /// The archive metadata used by the [ArchiveQueue].
        #[derive(Debug)]
        pub(super) struct ArchiveQueueMd {
            /// The database primary id of the weather location.
            pub lid: i64,
            /// The weather location alias name.
            pub alias: String,
            /// The weather data archive.
            pub file: WeatherFile,
        }

        /// A thread-safe collection of weather archive metadata used by the [ArchiveLoader].
        #[derive(Debug)]
        pub(super) struct ArchiveQueue(Mutex<Vec<ArchiveQueueMd>>);
        impl ArchiveQueue {
            pub fn new(conn: &Connection, weather_dir: &WeatherDir) -> Result<Self> {
                let id_alias_files: Vec<ArchiveQueueMd> = locations::id_aliases(conn)?
                    .into_iter()
                    .map(|(lid, alias)| {
                        let file = weather_dir.archive(&alias);
                        ArchiveQueueMd { lid, alias, file }
                    })
                    .collect();
                Ok(Self(Mutex::new(id_alias_files)))
            }
            pub fn next(&self) -> Option<ArchiveQueueMd> {
                match self.0.lock() {
                    Ok(mut guard) => guard.pop(),
                    Err(err) => err.into_inner().pop(),
                }
            }
        }
    }
}
