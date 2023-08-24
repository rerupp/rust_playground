//! The database implementation of weather data.

use crate::{
    backend::{self, DataAdapter, Error, Result},
    entities,
};
// use rusqlite as sql;
use rusqlite::Connection;

// Since database functionality is scoped to this module it's okay to add the error handler
// here and not in the module where Error is defined.
impl From<rusqlite::Error> for Error {
    /// Add support to convert rusqlite database errors.
    fn from(err: rusqlite::Error) -> Self {
        Error::from(format!("sql: {}", err))
    }
}

/// Export the function that will create the database [DataAdapter] adapter.
pub(crate) use v1::db_data_adapter as create_db_data_adapter;

pub(crate) use v1::admin;
mod v1 {
    //! The first version of the database implementation.
    use super::*;
    use backend::filesys::{weather_dir, ArchiveMd, WeatherArchive, WeatherDir};

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
            db_connection(Some($weather_dir.get_file(DB_FILENAME).to_string().as_str()))
        };
    }

    /// Create a [`DataAdapter`] based on the database configuration.
    ///
    /// # Arguments
    ///
    /// `dirname` is the directory containing weather data.
    pub(crate) fn db_data_adapter(dirname: &str) -> Result<Box<dyn DataAdapter>> {
        let weather_dir = weather_dir(dirname)?;
        let conn = db_conn!(&weather_dir)?;
        let db_config = admin::database_configuration(&conn)?;
        if db_config.hybrid {
            hybrid::create(weather_dir)
        } else if db_config.document {
            document::create(weather_dir)
        } else {
            normalized::create(weather_dir)
        }
    }

    pub(crate) mod admin {
        //! The implementation of weather data adminstation of a database.

        use super::*;
        use backend::filesys::WeatherFile;
        use entities::DbConfig;

        /// Initialize the database schema.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory.
        /// * `db_config` is the database configuration.
        /// * `drop` when true will delete the schema before intialization.
        /// * `load` when true will load weather data into the database.
        pub(crate) fn init_db(weather_dir: &WeatherDir, db_config: DbConfig, drop: bool, load: bool) -> Result<()> {
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
                    document::load(&mut conn, weather_dir, db_config.compress)?;
                } else {
                    normalized::load(&mut conn, weather_dir)?;
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
            let db_file = weather_dir.get_file(DB_FILENAME);
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
        pub(crate) fn stat(weather_dir: &WeatherDir) -> Result<DbConfig> {
            log::debug!("stat DB");
            let conn = db_conn!(weather_dir)?;
            let mode = database_configuration(&conn)?;
            Ok(mode)
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
                    DbConfig::full()
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
            let sql = r#"INSERT INTO config (hybrid, document, full, compress) VALUES (?1, ?2, ?3, ?4)"#;
            let params = (db_config.hybrid, db_config.document, db_config.full, db_config.compress);
            match conn.execute(sql, params) {
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
                let weather_dir = WeatherDir::new(&fixture.to_string()).unwrap();
                let db_file = PathBuf::from(&weather_dir.to_string()).join(DB_FILENAME);
                assert!(!db_file.exists());
                // hybrid
                admin::init_db(&weather_dir, DbConfig::hybrid(), true, true).unwrap();
                assert!(db_file.exists());
                let db_config = admin::stat(&weather_dir).unwrap();
                assert!(db_config.hybrid);
                assert!(!db_config.document);
                assert!(!db_config.full);
                assert!(!db_config.compress);
                admin::drop_db(&weather_dir, true).unwrap();
                assert!(!db_file.exists());
                // document uncompressed
                admin::init_db(&weather_dir, DbConfig::document(false), false, true).unwrap();
                assert!(db_file.exists());
                let db_config = admin::stat(&weather_dir).unwrap();
                assert!(!db_config.hybrid);
                assert!(db_config.document);
                assert!(!db_config.full);
                assert!(!db_config.compress);
                // document compressed
                admin::drop_db(&weather_dir, false).unwrap();
                admin::init_db(&weather_dir, DbConfig::document(true), false, true).unwrap();
                assert!(db_file.exists());
                let db_config = admin::stat(&weather_dir).unwrap();
                assert!(!db_config.hybrid);
                assert!(db_config.document);
                assert!(!db_config.full);
                assert!(db_config.compress);
                // full
                admin::drop_db(&weather_dir, false).unwrap();
                admin::init_db(&weather_dir, DbConfig::full(), false, false).unwrap();
                assert!(db_file.exists());
                let db_config = admin::stat(&weather_dir).unwrap();
                assert!(!db_config.hybrid);
                assert!(!db_config.document);
                assert!(db_config.full);
                assert!(!db_config.compress);
            }
        }
    }

    mod query {
        //! The common weather database queries.

        use super::*;
        use chrono::NaiveDate;
        use entities::{DataCriteria, DateRange, HistoryDates, HistorySummaries};
        use rusqlite::named_params;

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
                    compressed_size: None,
                })
                .collect();
            let aliases: Vec<&str> = history_summaries.iter().map(|h| h.location.alias.as_str()).collect();
            for (alias, count, compressed_size, raw_size) in query_history_summaries(conn, aliases)? {
                for history in &mut history_summaries {
                    if history.location.alias == alias {
                        history.count = count;
                        history.raw_size = Some(raw_size);
                        history.compressed_size = Some(compressed_size);
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
            // get the overall size of history for the table
            let table_size = query::sqlite_history_size(conn, table_name)?;
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
                let alias: String = row.get("alias")?;
                let location = Location {
                    id: alias.to_lowercase(),
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

            fn testenv() -> Connection {
                let fixture = testlib::TestFixture::create();
                let test_files = testlib::test_resources().join("db");
                fixture.copy_resources(&test_files);
                let weather_dir = WeatherDir::new(&fixture.to_string()).unwrap();
                admin::init_db(&weather_dir, DbConfig::hybrid(), true, true).unwrap();
                db_conn!(&weather_dir).unwrap()
            }

            #[test]
            fn query_locations() {
                let conn = testenv();
                let locations = get(&conn, &vec![], true).unwrap();
                assert_eq!(locations.len(), 3);
                for (location, expected_id) in locations.iter().zip(["between", "north", "south"].iter()) {
                    assert_eq!(location.id, *expected_id);
                }
                let locations = get(&conn, &vec!["south".to_string(), "north".to_string()], true).unwrap();
                assert_eq!(locations.len(), 2);
                for (location, expected_id) in locations.iter().zip(["north", "south"].iter()) {
                    assert_eq!(location.id, *expected_id);
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
        VALUES (?1, ?2, ?3, ?4, ?5)
    "#;

    /// The database hybrid implementation
    mod hybrid {
        #![allow(unused)]
        use super::*;
        use crate::{
            backend::{
                filesys::{archive_name, WeatherHistory},
                DataAdapter, Error, Result,
            },
            prelude::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location},
        };

        /// Create the *hybrid* version of the data adapter.
        pub(crate) fn create(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
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
            /// * `criteria` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories> {
                let mut locations = self.locations(criteria)?;
                match locations.len() {
                    0 => Err(Error::from("The data criteria did not result in finding a location.")),
                    1 => {
                        let location = locations.pop().unwrap();
                        let file = self.0.get_file(&archive_name(&location.alias));
                        let archive = WeatherHistory::new(&location.alias, file)?;
                        let daily_histories = archive.daily_histories(&history_range)?;
                        Ok(DailyHistories { location, daily_histories })
                    }
                    _ => Err(Error::from("The data criteria found more than 1 location.")),
                }
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
                    let file = self.0.get_file(&archive_name(&history_summary.location.alias));
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
                    let filename = archive_name(&alias);
                    let file = weather_dir.get_file(&filename);
                    let archive = WeatherArchive::open(&alias, file)?;
                    for md in archive.archive_iter(None, false, ArchiveMd::new)? {
                        let params = (lid, &md.date, md.compressed_size, md.size, md.mtime);
                        stmt.execute(params)?;
                    }
                }
            }
            tx.commit()?;
            Ok(())
        }
    }

    /// The database document implementation
    mod document {
        use super::*;
        use crate::{
            backend::{
                filesys::{archive_name, ArchiveData},
                DataAdapter, Error, Result,
            },
            prelude::{
                DailyHistories, DailyHistory, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location,
            },
        };
        use chrono::NaiveDate;
        use rusqlite::{blob::ZeroBlob, named_params, Transaction};
        use toolslib::stopwatch::StopWatch;

        /// Create the *document* version of the data adapter.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory name.
        pub(crate) fn create(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
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
            /// * `criteria` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, criteria: DataCriteria, date_range: DateRange) -> Result<DailyHistories> {
                let mut locations = self.locations(criteria)?;
                match locations.len() {
                    0 => Err(Error::from("The data criteria did not result in finding a location.")),
                    1 => {
                        let location = locations.pop().unwrap();
                        let conn = db_conn!(&self.0)?;
                        let daily_histories = query_daily_history(&conn, &location.alias, date_range)?;
                        Ok(DailyHistories { location, daily_histories })
                    }
                    _ => Err(Error::from("The data criteria found more than 1 location.")),
                }
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
                for (alias, overall_size) in query::table_size(&conn, "document")? {
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
        pub fn query_daily_history(conn: &Connection, alias: &str, date_range: DateRange) -> Result<Vec<DailyHistory>> {
            let db_config = admin::database_configuration(&conn)?;
            let mut stmt = conn.prepare(SELECT_SQL)?;
            let mut rows =
                stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
            let mut daily_histories = vec![];
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get("date")?;
                let json_text: String = if db_config.compress {
                    let rid: i64 = row.get("document_id")?;
                    let data = document::read_blob(conn, "documents", "daily_zip", rid)?;
                    let json = document::uncompress_str(&data[..])?;
                    json
                } else {
                    row.get("daily")?
                };
                let json = backend::filesys::to_json(json_text.as_bytes())?;
                daily_histories.push(backend::filesys::to_daily_history(alias, date, &json)?);
            }
            Ok(daily_histories)
        }

        use snap::{read, write};
        use std::io::{Read, Write};

        /// Compress a string using `snap`.
        ///
        /// # Arguments
        ///
        /// * `data` is the string that will be compressed.
        fn compress_str(data: &str) -> Result<Vec<u8>> {
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
        fn uncompress(compressed_data: &[u8]) -> Result<Vec<u8>> {
            let mut data = vec![];
            match read::FrameDecoder::new(&compressed_data[..]).read_to_end(&mut data) {
                Ok(_) => Ok(data),
                Err(err) => {
                    let reason = format!("Error reading compressed data ({})", err);
                    Err(Error::from(reason))
                }
            }
        }

        /// Writes a *blob* into the database. This is specific to `sqlite3`.
        ///
        /// # Arguments
        ///
        /// * `tx` is the transaction used to write to the database.
        /// * `table` is the table that will hold the *blob*.
        /// * `column` is the database column defined as a *blob*.
        /// * `rid` is the row identifier of the *blob*.
        fn write_blob(tx: &Transaction, history: &[u8], table: &str, column: &str, rid: i64) -> Result<()> {
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
        pub(super) fn read_blob(conn: &Connection, table: &str, column: &str, rid: i64) -> Result<Vec<u8>> {
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

        /// Loads the database based on the *document* implementation of weather data.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `weather_dir` is the weather data directory.
        /// * `compress` when true will compress the weather history documents.
        pub(super) fn load(conn: &mut Connection, weather_dir: &WeatherDir, compress: bool) -> Result<()> {
            log::info!("  documents");
            let mut histories_loaded: usize = 0;
            let mut overall_time = StopWatch::start_new();
            for (lid, alias) in locations::id_aliases(conn)? {
                let mut archive_load = StopWatch::start_new();
                let file = weather_dir.get_file(&archive_name(&alias));
                let archive = WeatherArchive::open(&alias, file)?;
                let mut tx = conn.transaction()?;
                let history_count = load_archive(&mut tx, lid, &archive, compress)?;
                tx.commit()?;
                archive_load.stop();
                let per_msec = ((history_count as f64 / archive_load.elapsed().as_millis() as f64) * 1000.0) as usize;
                log::debug!("    {}: {} loaded in: {} ({}us/history)", alias, history_count, archive_load, per_msec);
                histories_loaded += history_count;
            }
            overall_time.stop();
            let per_msec = ((histories_loaded as f64 / overall_time.elapsed().as_millis() as f64) * 1000.0) as usize;
            log::debug!("    {} histories load time: {} ({}us/history)", histories_loaded, overall_time, per_msec);
            Ok(())
        }

        /// Load the contents of a weather data archive into the databaase.
        ///
        /// # Arguments
        ///
        /// * `tx` is the database transaction that will be used.
        /// * `lid` is the location primary id history is associated with.
        /// * `archive` is the weather history archive to load.
        /// * `compress` when true will compress the weather history documents.
        fn load_archive(tx: &mut Transaction, lid: i64, archive: &WeatherArchive, compress: bool) -> Result<usize> {
            const DATA_SQL: &str =
                r#"INSERT INTO documents (mid, daily, daily_zip, daily_size) VALUES (?1, ?2, ?3, ?4)"#;
            let mut data_stmt = tx.prepare_cached(DATA_SQL)?;
            let mut md_stmt = tx.prepare_cached(METADATA_SQL)?;
            let mut histories_loaded = 0;
            for data in archive.archive_iter(None, false, ArchiveData::new)? {
                let json = data.json()?;
                let daily = &json["daily"]["data"][0].to_string();
                if compress {
                    let daily_zip = document::compress_str(daily)?;
                    let size = daily.len();
                    let store_size = daily_zip.len();
                    md_stmt.execute((lid, data.date, store_size, size, 0))?;
                    let mid = tx.last_insert_rowid();
                    data_stmt.execute((mid, "", ZeroBlob(daily_zip.len() as i32), daily.len()))?;
                    let rid = tx.last_insert_rowid();
                    document::write_blob(tx, &daily_zip, "documents", "daily_zip", rid)?;
                } else {
                    let size = daily.len();
                    md_stmt.execute((lid, data.date, size, size, 0))?;
                    let mid = tx.last_insert_rowid();
                    data_stmt.execute((mid, daily, ZeroBlob(0), daily.len()))?;
                }
                histories_loaded += 1;
            }
            Ok(histories_loaded)
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
                let compressed = compress_str(testcase).unwrap();
                // now insert the data
                let size = compressed.len();
                let row_id;
                {
                    let tx = conn.transaction().unwrap();
                    let insert = "INSERT INTO example (mid, data) VALUES (?1, ?2)";
                    tx.execute(insert, (1, ZeroBlob(size as i32))).unwrap();
                    row_id = tx.last_insert_rowid();
                    write_blob(&tx, &compressed[..], "example", "data", row_id).unwrap();
                    tx.commit().unwrap();
                }
                let compressed_data = read_blob(&conn, "example", "data", row_id).unwrap();
                assert_eq!(compressed, compressed_data);
                let uncompressed_data = uncompress_str(&compressed_data[..]).unwrap();
                assert_eq!(testcase, uncompressed_data);
            }
        }
    }

    mod normalized {
        //! The [DataAdapter] implementation using a normalized datbase schema.
        use super::*;
        use crate::{
            backend::{
                filesys::{archive_name, ArchiveData},
                DataAdapter, Error, Result,
            },
            prelude::{
                DailyHistories, DailyHistory, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location,
            },
        };
        use rusqlite::{named_params, Transaction};
        use toolslib::stopwatch::StopWatch;

        /// Create the *normalized* version of the data adapter.
        ///
        /// # Arguments
        ///
        /// * `weather_dir` is the weather data directory name.
        pub(crate) fn create(weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
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
            /// * `criteria` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, criteria: DataCriteria, date_range: DateRange) -> Result<DailyHistories> {
                let mut locations = self.locations(criteria)?;
                match locations.len() {
                    0 => Err(Error::from("The data criteria did not result in finding a location.")),
                    1 => {
                        let location = locations.pop().unwrap();
                        let conn = db_conn!(&self.0)?;
                        let daily_histories = query_daily_history(&conn, &location.alias, date_range)?;
                        Ok(DailyHistories { location, daily_histories })
                    }
                    _ => Err(Error::from("The data criteria found more than 1 location.")),
                }
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
                for (alias, overall_size) in query::table_size(&conn, "daily")? {
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

        /// The `SQL` used to insert normalized data into the database.
        const INSERT_SQL: &str = r#"
            INSERT INTO daily (
                mid,
                temp_high,
                temp_high_t,
                temp_low,
                temp_low_t,
                temp_max,
                temp_max_t,
                temp_min,
                temp_min_t,
                wind_speed,
                wind_gust,
                wind_gust_t,
                wind_bearing,
                cloud_cover,
                uv_index,
                uv_index_t,
                summary,
                humidity,
                dew_point,
                sunrise_t,
                sunset_t,
                moon_phase
            )
            VALUES (
                :mid,
                :temp_high,
                :temp_high_t,
                :temp_low,
                :temp_low_t,
                :temp_max,
                :temp_max_t,
                :temp_min,
                :temp_min_t,
                :wind_speed,
                :wind_gust,
                :wind_gust_t,
                :wind_bearing,
                :cloud_cover,
                :uv_index,
                :uv_index_t,
                :summary,
                :humidity,
                :dew_point,
                :sunrise_t,
                :sunset_t,
                :moon_phase
            )"#;

        /// Loads the database based on the *normalized* implementation of weather data.
        ///
        /// # Arguments
        ///
        /// * `conn` is the database connection that will be used.
        /// * `weather_dir` is the weather data directory.
        pub(super) fn load(conn: &mut Connection, weather_dir: &WeatherDir) -> Result<()> {
            log::info!("  full");
            let size_estimate = size_estimate(conn, "daily")?;
            let mut histories_loaded: usize = 0;
            let mut overall_time = StopWatch::start_new();
            for (lid, alias) in locations::id_aliases(conn)? {
                let mut archive_load = StopWatch::start_new();
                let file = weather_dir.get_file(&archive_name(&alias));
                let archive = WeatherArchive::open(&alias, file)?;
                let mut tx = conn.transaction()?;
                let history_count = load_archive(&mut tx, lid, &archive, size_estimate)?;
                tx.commit()?;
                archive_load.stop();
                let per_msec = ((history_count as f64 / archive_load.elapsed().as_millis() as f64) * 1000.0) as usize;
                log::debug!("    {}: {} loaded in: {} ({}us/history)", alias, history_count, archive_load, per_msec);
                histories_loaded += history_count;
            }
            overall_time.stop();
            let per_msec = ((histories_loaded as f64 / overall_time.elapsed().as_millis() as f64) * 1000.0) as usize;
            log::debug!("    {} histories load time: {} ({}us/history)", histories_loaded, overall_time, per_msec);
            Ok(())
        }

        /// Load the contents of a weather data archive into the databaase.
        ///
        /// # Arguments
        ///
        /// * `tx` is the database transaction that will be used.
        /// * `lid` is the location primary id history is associated with.
        /// * `archive` is the weather history archive to load.
        /// * `fixed_size_estimate` is an estimate of the table overhead to determine the size.
        fn load_archive(
            tx: &mut Transaction,
            lid: i64,
            archive: &WeatherArchive,
            fixed_size_estimate: usize,
        ) -> Result<usize> {
            let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
            let mut md_stmt = tx.prepare_cached(METADATA_SQL)?;
            let mut histories_loaded = 0;
            for data in archive.archive_iter(None, false, ArchiveData::new)? {
                let json = data.json()?;
                let daily = &json["daily"]["data"][0];
                if !daily.is_object() {
                    // not pleased with this pattern but its good enough for now
                    let reason = format!("{}: Did not find daily history for {}", data.lid, &data.date);
                    return Err(Error::from(reason));
                }
                let size = daily.to_string().len();
                let summary = daily.get("summary").map_or(None, |v| v.as_str());
                let store_size = fixed_size_estimate + summary.map_or(0, |s| s.len());
                md_stmt.execute((lid, &data.date, store_size, size, 0))?;
                let mid = tx.last_insert_rowid();
                data_stmt.execute(named_params! {
                    ":mid": mid,
                    ":temp_high": daily.get("temperatureHigh").map_or(None, |v| v.as_f64()),
                    ":temp_high_t": daily.get("temperatureHighTime").map_or(None, |v| v.as_i64()),
                    ":temp_low": daily.get("temperatureLow").map_or(None, |v| v.as_f64()),
                    ":temp_low_t": daily.get("temperatureLowTime").map_or(None, |v| v.as_i64()),
                    ":temp_max": daily.get("temperatureMax").map_or(None, |v| v.as_f64()),
                    ":temp_max_t": daily.get("temperatureMaxTime").map_or(None, |v| v.as_i64()),
                    ":temp_min": daily.get("temperatureMin").map_or(None, |v| v.as_f64()),
                    ":temp_min_t": daily.get("temperatureMinTime").map_or(None, |v| v.as_i64()),
                    ":wind_speed": daily.get("windSpeed").map_or(None, |v| v.as_f64()),
                    ":wind_gust": daily.get("windGust").map_or(None, |v| v.as_f64()),
                    ":wind_gust_t": daily.get("windGustTime").map_or(None, |v| v.as_i64()),
                    ":wind_bearing": daily.get("windBearing").map_or(None, |v| v.as_i64()),
                    ":cloud_cover": daily.get("cloudCover").map_or(None, |v| v.as_f64()),
                    ":uv_index": daily.get("uvIndex").map_or(None, |v| v.as_i64()),
                    ":uv_index_t": daily.get("uvIndexTime").map_or(None, |v| v.as_i64()),
                    ":summary": daily.get("summary").map_or(None, |v| v.as_str()),
                    ":humidity": daily.get("humidity").map_or(None, |v| v.as_f64()),
                    ":dew_point": daily.get("dewPoint").map_or(None, |v| v.as_f64()),
                    ":sunrise_t": daily.get("sunriseTime").map_or(None, |v| v.as_i64()),
                    ":sunset_t": daily.get("sunsetTime").map_or(None, |v| v.as_i64()),
                    ":moon_phase": daily.get("moonPhase").map_or(None, |v| v.as_f64()),
                })?;
                histories_loaded += 1;
            }
            Ok(histories_loaded)
        }

        /// The `SQL` used to select data from the database.
        const SELECT_SQL: &str = r#"
        SELECT
            l.id AS lid,
            m.date AS date,
            d.temp_high AS temp_high,
            d.temp_high_t AS temp_high_t,
            d.temp_low AS temp_low,
            d.temp_low_t AS temp_low_t,
            d.temp_max AS temp_max,
            d.temp_max_t AS temp_max_t,
            d.temp_min AS temp_min,
            d.temp_min_t AS temp_min_t,
            d.wind_speed AS wind_speed,
            d.wind_gust AS wind_gust,
            d.wind_gust_t AS wind_gust_t,
            d.wind_bearing AS wind_bearing,
            d.cloud_cover AS cloud_cover,
            d.uv_index AS uv_index,
            d.uv_index_t AS uv_index_t,
            d.summary AS summary,
            d.humidity AS humidity,
            d.dew_point AS dew_point,
            d.sunrise_t AS sunrise_t,
            d.sunset_t AS sunset_t,
            d.moon_phase AS moon_phase
        FROM locations l
            INNER JOIN metadata AS m ON l.id=m.lid
            INNER JOIN daily AS d ON m.id=d.mid
        WHERE
            l.alias=:alias AND m.date BETWEEN :from AND :thru
        ORDER BY date
        "#;

        /// Get daily history from the database.
        /// 
        /// # Arguments
        /// 
        /// * `conn` is the database connection that will be used.
        /// * `alias` is the location alias name.
        /// * `date_range` determines the daily history.
        fn query_daily_history(conn: &Connection, alias: &str, date_range: DateRange) -> Result<Vec<DailyHistory>> {
            // let mut stmt = conn.prepare(include_str!("db/daily.sql"))?;
            let mut stmt = conn.prepare(SELECT_SQL)?;
            let mut rows =
                stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
            let mut daily_histories = vec![];
            while let Some(row) = rows.next()? {
                daily_histories.push(DailyHistory {
                    location_id: alias.to_string(),
                    date: row.get("date")?,
                    temperature_high: row.get("temp_high")?,
                    temperature_high_time: row.get("temp_high_t")?,
                    temperature_low: row.get("temp_low")?,
                    temperature_low_time: row.get("temp_low_t")?,
                    temperature_max: row.get("temp_max")?,
                    temperature_max_time: row.get("temp_max_t")?,
                    temperature_min: row.get("temp_min")?,
                    temperature_min_time: row.get("temp_min_t")?,
                    wind_speed: row.get("wind_speed")?,
                    wind_gust: row.get("wind_gust")?,
                    wind_gust_time: row.get("wind_gust_t")?,
                    wind_bearing: row.get("wind_bearing")?,
                    cloud_cover: row.get("cloud_cover")?,
                    uv_index: row.get("uv_index")?,
                    uv_index_time: row.get("uv_index_t")?,
                    summary: row.get("summary")?,
                    humidity: row.get("humidity")?,
                    dew_point: row.get("dew_point")?,
                    sunrise_time: row.get("sunrise_t")?,
                    sunset_time: row.get("sunset_t")?,
                    moon_phase: row.get("moon_phase")?,
                })
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
            let mut size_estimate = 0;
            // this is specific to sqlite3
            conn.pragma(None, table, "daily", |row| {
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
    }
}
