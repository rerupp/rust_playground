//! This module manages the metadata surrounding weather data history.
use super::*;

pub(super) const TABLE_NAME: &str = "metadata";
pub(super) use v3::{insert, row_size, delete, examine_add_histories};
mod v3 {
    //! The current version of weather data history metadata.
    use super::*;
    use chrono::NaiveDate;
    use toolslib::stopwatch::StopWatch;

    /// The metadata insert SQL used by the [DataAdapter] implementations.
    const METADATA_SQL: &str = r#"
    INSERT INTO metadata (lid, date, store_size, size, mtime)
        VALUES (:lid, :date, :store_size, :size, :mtime)
    "#;

    /// Insert metadata into the database.
    ///
    /// # Arguments
    ///
    /// * `tx` is the transaction that will be used to insert data.
    /// * `lid` is the location primary id.
    /// * `date` is the history date.
    /// * `store_size` is the size of data in the database.
    /// * `size` is the size of history data.
    pub fn insert(tx: &Transaction, lid: i64, date: &NaiveDate, store_size: usize, size: usize) -> Result<i64> {
        let mut stmt = tx.prepare(METADATA_SQL)?;
        stmt.execute(named_params! {
            ":lid": lid,
            ":date": date,
            ":store_size": store_size,
            ":size": size,
            ":mtime": 0
        })?;
        Ok(tx.last_insert_rowid())
    }

    /// Get the size of a row in the metadata table.
    pub fn row_size() -> usize {
        // for the current table this is the best guess size
        30
    }

    /// Remove all metadata associated with the location id.
    ///
    /// # Arguments
    ///
    /// * `tx` is the database transaction that will be used.
    /// * `lid` is the location id.
    pub fn delete(tx: &Transaction, lid: i64) -> Result<()> {
        const SQL: &str = "DELETE FROM metadata where lid=:lid";
        let mut stmt = tx.prepare(SQL)?;
        stmt.execute(named_params! {":lid": lid})?;
        Ok(())
    }

    /// Find the weather histories that are not already in the metadata.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection to use.
    /// * `lid` is the location id.
    /// * `daily_histories` holds the location histories to audit.
    pub fn examine_add_histories<'h>(
        conn: &Connection,
        daily_histories: &'h DailyHistories,
    ) -> Result<(i64, Vec<&'h History>)> {
        let stopwatch = StopWatch::start_new();
        // get the history dates for the location
        let alias = daily_histories.location.alias.as_str();
        let histories = &daily_histories.histories;
        let lid = locations::location_id(conn, alias)?;
        const SQL: &str = r#"SELECT date FROM metadata WHERE lid = :lid"#;
        let mut stmt = conn.prepare(&SQL)?;
        let mut rows = stmt.query(named_params! {":lid": lid})?;
        // find which histories already exist
        let mut already_exists: Vec<NaiveDate> = Vec::with_capacity(histories.len());
        while let Some(row) = rows.next()? {
            let date: NaiveDate = row.get("date")?;
            if histories.iter().any(|history| history.date == date) {
                already_exists.push(date);
                // you're done if all the histories already exist
                if already_exists.len() == histories.len() {
                    break;
                }
            }
        }
        // filter out the histories that already exist
        let okay_to_add = histories
            .iter()
            .filter_map(|history| match already_exists.iter().any(|date| &history.date == date) {
                true => None,
                false => Some(history),
            })
            .collect();
        // show the locations history dates that already exist
        if already_exists.len() > 0 {
            let dates = already_exists.iter().map(|date| date.to_string()).collect::<Vec<String>>().join(", ");
            log::info!("audit_add_histories: Location '{}' already has these history dates {}.", alias, dates);
        }
        log::debug!("audit_add_histories {}", stopwatch);
        Ok((lid, okay_to_add))
    }
}
