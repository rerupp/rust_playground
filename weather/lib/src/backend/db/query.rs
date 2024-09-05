//! The common weather database queries.
use super::*;

// pub(in crate::backend) use v3::{db_size, history_dates, history_counts, DbSizes, HistoryCounts};
pub(in crate::backend) use v3::{db_size, history_dates, history_counts};
mod v3 {
    //! The current implementation of weather data queries.
    use super::*;
    use chrono::NaiveDate;

    /// Get the location history dates.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
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
        // log::trace!("{}", sql);
        // execute the query
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
    /// * `conn` is the database connection that will be used.
    /// * `criteria` is the location data criteria.
    #[allow(unused)]
    pub fn history_sizes(conn: &Connection, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
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
        for (alias, count, store_size, raw_size) in query_history_sizes(conn, aliases)? {
            for history in &mut history_summaries {
                if history.location.alias == alias {
                    history.count = count;
                    history.raw_size = Some(raw_size);
                    history.store_size = Some(store_size);
                    break;
                }
            }
        }
        Ok(history_summaries)
    }

    /// Execute the query to get location history summaries.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection used to execute the query.
    /// * `aliases` is used to restrict what locations will be returned.
    fn query_history_sizes(conn: &Connection, aliases: Vec<&str>) -> Result<Vec<(String, usize, usize, usize)>> {
        // build the history summary query
        let mut sql = r#"
            SELECT
                l.alias AS alias,
                COUNT(m.date) AS count,
                SUM(m.store_size) AS store_size,
                SUM(m.size) AS size
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
        // log::trace!("{}", sql);
        // execute the query
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        let mut history_summaries: Vec<(String, usize, usize, usize)> = vec![];
        while let Some(row) = rows.next()? {
            let alias: String = row.get("alias")?;
            let count: usize = row.get("count")?;
            let store_size: usize = row.get("store_size")?;
            let size: usize = row.get("size")?;
            history_summaries.push((alias, count, store_size, size));
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
    pub fn db_size(conn: &Connection, table_name: &str) -> Result<DbSizes> {
        // get the count of history dates for each location
        let history_counts = history_counts(conn)?;
        let total = history_counts.0.iter().map(|(_, size)| size).sum::<usize>();
        // get the overall size of history in the database
        let table_size = if table_name == metadata::TABLE_NAME {
            sqlite_metadata_size(conn)?
        } else {
            sqlite_history_size(conn, table_name)?
        };
        // calculate the sizes based on the number of histories
        let locations_size: Vec<(String, usize)> = history_counts
            .0
            .into_iter()
            .map(|(alias, count)| {
                let percentage = count as f64 / total as f64;
                let size = (table_size as f64 * percentage) as usize;
                (alias, size)
            })
            .collect();
        Ok(DbSizes(locations_size))
    }

    /// The collection of location aliases and the size in the database.
    #[derive(Debug)]
    pub struct DbSizes(
        /// The location and database size tuples.
        Vec<(String, usize)>,
    );
    impl DbSizes {
        /// Get the size of history in the database for a location.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location alias name.
        pub fn get(&self, alias: &str) -> usize {
            let size = self.0.iter().find_map(|(table_alias, count)| match table_alias == alias {
                true => Some(*count),
                false => None,
            });
            if let Some(size) = size {
                size
            } else {
                log::warn!("Did not find table size for '{}'.", alias);
                0
            }
        }
    }

    /// Used internally to help calculate the amount of metadata space being used by locations.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    fn sqlite_metadata_size(conn: &Connection) -> Result<usize> {
        const SQL: &str = r#"
            SELECT
                SUM(pgsize) AS size
            FROM dbstat
                WHERE name LIKE '%metadata%'
            "#;
        let mut stmt = conn.prepare(SQL)?;
        let db_size = stmt.query_row([], |row| {
            let size: usize = row.get("size")?;
            Ok(size)
        })?;
        Ok(db_size)
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
    pub fn history_counts(conn: &Connection) -> Result<HistoryCounts> {
        const SQL: &str = r#"
            SELECT
                l.alias AS alias,
                COUNT(m.date) AS COUNT
            FROM locations AS l
                INNER JOIN metadata AS m ON l.id=m.lid
            GROUP BY l.alias
            ORDER BY l.alias
            "#;
        let mut counts: Vec<(String, usize)> = vec![];
        let mut stmt = conn.prepare(SQL)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let alias: String = row.get("alias")?;
            let count: usize = row.get("count")?;
            counts.push((alias, count));
        }
        Ok(HistoryCounts(counts))
    }

    /// The collection of location aliases and history counts.
    #[derive(Debug)]
    pub struct HistoryCounts(
        /// The location and history count tuples.
        Vec<(String, usize)>,
    );
    impl HistoryCounts {
        /// Get the history count for a location.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location alias name.
        pub fn get(&self, alias: &str) -> usize {
            let count = self.0.iter().find_map(|(inner_alias, count)| match inner_alias == alias {
                true => Some(*count),
                false => None,
            });
            if let Some(count) = count {
                count
            } else {
                log::warn!("Did not find history count for '{}'.", alias);
                0
            }
        }
    }
}
