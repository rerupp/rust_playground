//! Provide database support for weather data locations.
use super::*;

pub(super) use v3::{add, search, load, get, id_aliases, location_id};
mod v3 {
    //! The current version of locations for the various database modes.
    use super::*;
    use us_cities::UsCities;
    use entities::LocationCriteria;

    pub fn add(conn: &mut Connection, mut location: Location, weather_dir: &WeatherDir) -> Result<()> {
        // add the location to the location document to vet out any issues
        location.alias = location.alias.to_lowercase();
        let mut locations = filesys::weather_locations(weather_dir)?;
        locations.add(location.clone(), weather_dir)?;
        // now try to insert the location
        log::debug!("  add");
        let tx = conn.transaction()?;
        const SQL: &str =
            r#"INSERT INTO locations (name, alias, longitude, latitude, tz) VALUES (?1, ?2, ?3, ?4, ?5)"#;
        let mut stmt = tx.prepare(SQL)?;
        let alias = location.alias.clone();
        let params = (location.name, location.alias, location.longitude, location.latitude, location.tz);
        match stmt.execute(params) {
            Ok(_) => (),
            Err(err) => {
                let reason = format!("Error adding location '{}' ({}).", alias, &err);
                return Err(Error::from(reason));
            }
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    /// Search the US cities database for locations.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `criteria` is used to filter the search results.
    pub fn search(config: &Config, criteria: LocationCriteria) -> Result<Vec<Location>> {
        let us_cities = UsCities::try_from(config)?;
        us_cities.search(criteria)
    }

    /// Loads the location metadata document into the database.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `weather_dir` is the weather data directory.
    pub fn load(conn: &mut Connection, weather_dir: &WeatherDir) -> Result<()> {
        log::debug!("  locations");
        let locations = filesys::weather_locations(weather_dir)?;
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
    /// * `sort` when true the locations will be in location name order.
    pub fn get(conn: &Connection, filters: &Vec<String>, sort: bool) -> Result<Vec<Location>> {
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
        // log::trace!("{}", sql);
        sql
    }

    /// Get the location id and alias.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    pub fn id_aliases(conn: &Connection) -> Result<Vec<(i64, String)>> {
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

    /// Get the locations database identifier.
    ///
    /// # Arguments
    ///
    /// * `alias` is the location alias name.
    pub fn location_id(conn: &Connection, alias: &str) -> Result<i64> {
        let mut stmt = conn.prepare("SELECT id FROM locations AS l where l.alias = :alias")?;
        match stmt.query_row(named_params! {":alias": alias}, |row| Ok(row.get(0))) {
            Ok(id) => Ok(id.unwrap()),
            Err(err) => {
                let reason = format!("Error getting id for '{}' ({}).", alias, err);
                Err(Error::from(reason))
            }
        }
    }

//     use crate::backend::filesys;
    use conditions::{between, equals, like, or, Condition, Like, Or};
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

        #[allow(unused)]
        macro_rules! equals {
            ($column:expr, $value:expr) => {
                Condition::Equals(Equals::column($column).value($value))
            };
        }
        pub(super) use equals;

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

        macro_rules! like {
            ($column:expr, $value:expr) => {
                Condition::Like(Like::column($column).value($value))
            };
        }
        pub(super) use like;

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

        #[allow(unused)]
        macro_rules! between {
            ($column:expr, $from:expr, $thru:expr) => {
                Condition::Between(Between::column($column).from($from).thru($thru))
            };
        }
        pub(super) use between;

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

        macro_rules! or {
            ($conditions:expr) => {
                Condition::Or(Or::conditions($conditions))
            };
        }
        pub(super) use or;

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
        use crate::db_conn;

        fn testenv(fixture: &testlib::TestFixture) -> Connection {
            let test_files = testlib::test_resources().join("db");
            fixture.copy_resources(&test_files);
            let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
            admin::init_db(&weather_dir, DbMode::Hybrid, true, true, 1).unwrap();
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