//! The database document implementation.
use super::*;

// The name of the document table.
pub(super) const TABLE_NAME: &str = "documents";

pub(super) use v3::{data_adapter, loader::load, reload};
mod v3 {
    //! The current implementation of the document database.
    use super::*;
    use rusqlite::{blob::ZeroBlob, types::Null};

    /// Create the *document* version of the data adapter.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory name.
    /// * `compressed` when true will compress the weather history `JSON` documents.
    pub fn data_adapter(config: Config, weather_dir: WeatherDir, compressed: bool) -> Result<Box<dyn DataAdapter>> {
        log::debug!("Document data adapter.");
        let data_adapter = DocumentDataAdapter { config, weather_dir, compress: compressed };
        Ok(Box::new(data_adapter))
    }

    /// The *document* data adapter implementation.
    #[derive(Debug)]
    struct DocumentDataAdapter {
        config: Config,
        /// The weather data directory.
        weather_dir: WeatherDir,
        /// When `true` weather history `JSON` documents will be compressed in the database
        compress: bool,
    }
    impl DataAdapter for DocumentDataAdapter {
        /// get the document data adapter configuration.
        fn config(&self) -> &Config {
            &self.config
        }
        /// Add weather data history for a location.
        ///
        /// # Arguments
        ///
        /// * `daily_histories` has the location and histories to add.
        fn add_daily_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
            // add histories to the archive
            let file_histories = archive::add_histories(&self.weather_dir, daily_histories)?;
            // find which histories can be added to the database
            let mut conn = db_conn!(&self.weather_dir)?;
            let (lid, histories) = metadata::examine_add_histories(&conn, daily_histories)?;
            // add histories to the db
            let base_size = size_estimate(&conn, TABLE_NAME)? + metadata::row_size();
            let mut tx = conn.transaction()?;
            for history in &histories {
                insert_history(&mut tx, lid, history, base_size, 0, self.compress)?;
            }
            tx.commit()?;
            // report any differences adding to the archive and db
            let histories_added = cmp::max(file_histories.len(), histories.len());
            self.audit_add_histories(&daily_histories.histories, file_histories, histories);
            Ok(histories_added)
        }
        /// Returns the daily weather data history for a location.
        ///
        /// # Arguments
        ///
        /// * `location` identifies what location should be used.
        /// * `history_range` specifies the date range that should be used.
        fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories> {
            let conn = db_conn!(&self.weather_dir)?;
            let daily_histories = query_daily_history(&conn, &location.alias, date_range, self.compress)?;
            Ok(DailyHistories { location, histories: daily_histories })
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
            let conn = db_conn!(&self.weather_dir)?;
            let db_sizes = query::db_size(&conn, TABLE_NAME)?;
            let history_counts = query::history_counts(&conn)?;
            let history_summaries = self
                .locations(criteria)?
                .into_iter()
                .map(|location| {
                    let db_size = db_sizes.get(&location.alias);
                    let count = history_counts.get(&location.alias);
                    let archive_size = archive::store_size(&self.weather_dir, &location.alias);
                    HistorySummaries {
                        location,
                        count,
                        overall_size: Some(db_size + archive_size),
                        raw_size: Some(db_size),
                        store_size: Some(archive_size),
                    }
                })
                .collect();
            Ok(history_summaries)
        }
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

    /// The `SQL` select statement used by the *document* implementation.
    const SELECT_SQL: &str = r#"
    SELECT
        l.id AS lid,
        m.date AS date,
        d.id as did,
        d.plain AS plain,
        d.size AS size
    FROM locations AS l
        INNER JOIN metadata AS m ON l.id=m.lid
        INNER JOIN documents AS d ON m.id=d.mid
    WHERE
        l.alias=:alias AND m.date BETWEEN :from AND :thru
    ORDER BY date
    "#;

    /// Query the database for daily history.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `alias` is the location alias.
    /// * `date_range` identifies what daily history will be returned.
    fn query_daily_history(
        conn: &Connection,
        alias: &str,
        date_range: DateRange,
        compressed: bool,
    ) -> Result<Vec<History>> {
        let mut stmt = conn.prepare(SELECT_SQL)?;
        let mut rows = stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
        let mut daily_histories = vec![];
        while let Some(row) = rows.next()? {
            let json_text: String = if compressed {
                let rid: i64 = row.get("did")?;
                let data = blob::read(conn, TABLE_NAME, "zipped", rid)?;
                let json = compression::uncompress_str(&data[..])?;
                json
            } else {
                row.get("plain")?
            };
            let history = history::from_bytes(alias, json_text.as_bytes())?;
            daily_histories.push(history);
        }
        Ok(daily_histories)
    }

    /// The `SQL` used to insert normalized data into the database.
    const INSERT_SQL: &str = r#"
    INSERT INTO documents (mid, plain, zipped, size)
        VALUES (:mid, :plain, :zipped, :size)
    "#;

    /// Insert history documents into the database.
    ///
    /// # Arguments
    ///
    /// * `tx` is the transaction that will be used to insert data.
    /// * `lid` is the location primary id.
    /// * `history` is what will be inserted as a `JSON` document.
    /// * `base_size` is the database size minus the history document len.
    /// * `store_size` is the history data size in the archive.
    /// * `compress` indicates the document should be compressed.
    fn insert_history(
        tx: &mut Transaction,
        lid: i64,
        history: &History,
        base_size: usize,
        store_size: usize,
        compress: bool,
    ) -> Result<()> {
        let json = history::to_json(history)?;
        let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
        if compress {
            let compressed_json = compression::compress(json.as_bytes())?;
            let compressed_size = compressed_json.len();
            let mid = metadata::insert(tx, lid, &history.date, store_size, base_size + compressed_size)?;
            data_stmt.execute(named_params! {
                ":mid": mid,
                ":plain": Null,
                ":zipped": ZeroBlob(compressed_size as i32),
                ":size": compressed_size
            })?;
            let did = tx.last_insert_rowid();
            blob::write(tx, &compressed_json, TABLE_NAME, "zipped", did)?;
        } else {
            let json_len = json.len();
            let mid = metadata::insert(tx, lid, &history.date, store_size, base_size + json_len)?;
            data_stmt.execute(named_params! {
                ":mid": mid,
                ":plain": json,
                ":zipped": Null,
                ":size": json_len
            })?;
        }
        Ok(())
    }

    /// Reload a locations weather history for the *document* implementation of weather data.
    ///
    /// # Argument
    ///
    /// * `conn` is the database connection that will be used.
    /// * `weather_dir` is the weather data directory.
    /// * `alias` is the location that will be reloaded.
    /// * `compress` determines if history data will be compressed in the database.
    pub fn reload(conn: &mut Connection, weather_dir: &WeatherDir, alias: &str, compress: bool) -> Result<()> {
        let stopwatch = StopWatch::start_new();
        let base_size = size_estimate(&conn, TABLE_NAME)?;
        let lid = locations::location_id(conn, alias)?;
        let archive = WeatherArchive::open(alias, weather_dir.archive(alias))?;
        const DELETE_SQL: &str = r#"
        DELETE FROM documents
        WHERE ROWID IN (
          SELECT d.ROWID FROM documents AS d
          INNER JOIN metadata AS m ON d.mid = m.id
          WHERE m.lid = :lid
        )
        "#;
        let mut tx = conn.transaction()?;
        let mut stmt = tx.prepare(DELETE_SQL)?;
        stmt.execute(named_params! {":lid": lid})?;
        drop(stmt);
        metadata::delete(&tx, lid)?;
        for (md, history) in archive.iter_date_range(None, false, archive_history_collector)? {
            insert_history(&mut tx, lid, &history, base_size, md.compressed_size as usize, compress)?;
        }
        tx.commit()?;
        log::debug!("reload {} in {}", alias, stopwatch);
        Ok(())
    }

    // pub(crate) use loader::load;
    pub mod loader {
        //! The document database archive loader for weather data.
        use super::*;
        use archive::loader::*;
        use std::{
            sync::mpsc::{Receiver, Sender, TryRecvError},
            thread, time,
        };

        /// The data passed through the [ArchiveLoader].
        #[derive(Debug)]
        struct LoadMsg {
            /// The location identifier.
            lid: i64,
            /// The archive history metadata.
            md: ArchiveMd,
            /// The daily history.
            history: History,
        }

        /// Take the [History] archives and push them into the database.
        ///
        /// # Argument
        ///
        /// * `weather_dir` is the weather data directory.
        /// * `compress` determines if history data will be compressed in the database.
        /// * `threads` is the number of workers to use getting data from archives.
        pub fn load(weather_dir: &WeatherDir, compress: bool, threads: usize) -> Result<()> {
            let conn = db_conn!(weather_dir)?;
            let size_estimate = size_estimate(&conn, TABLE_NAME)?;
            let archives = ArchiveQueue::new(&conn, weather_dir)?;
            let mut loader: ArchiveLoader<LoadMsg> = ArchiveLoader::new(threads);
            loader.execute(
                archives,
                || Box::new(HistoryProducer),
                || Box::new(HistoryConsumer { conn, base_size: size_estimate, compress }),
            )
        }

        /// The [History] data producer.
        struct HistoryProducer;
        impl HistoryProducer {
            /// Send the history data to the consumer side of the loader.
            ///
            /// # Arguments
            ///
            /// * `lid` is the locations primary id in the database.
            /// * `md` is the archive history metadata.
            /// * `history` is the data that will be sent off to the consumer.
            /// * `sender` is used to pass data to the collector.
            fn send_history(&self, lid: i64, md: ArchiveMd, history: History, sender: &Sender<LoadMsg>) -> Result<()> {
                match sender.send(LoadMsg { lid, md, history }) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(Error::from("HistoryConsumer disconnected...")),
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
                let mut history_cnt = 0;
                for (md, history) in archive.iter_date_range(None, false, archive_history_collector)? {
                    self.send_history(lid, md, history, sender)?;
                    history_cnt += 1;
                }
                Ok(history_cnt)
            }
        }

        /// The document based history consumer.
        struct HistoryConsumer {
            /// The database connection that will be used
            conn: Connection,
            /// The base size estimate of the document table
            base_size: usize,
            /// Documents should be compressed
            compress: bool,
        }
        impl ArchiveConsumer<LoadMsg> for HistoryConsumer {
            /// The document *consumer* side of the archive data.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the gathered archive data.
            fn collect(&mut self, receiver: Receiver<LoadMsg>) -> Result<usize> {
                let mut tx = self.conn.transaction()?;
                let mut count: usize = 0;
                let pause = time::Duration::from_millis(1);
                loop {
                    match receiver.try_recv() {
                        Ok(msg) => {
                            insert_history(
                                &mut tx,
                                msg.lid,
                                &msg.history,
                                self.base_size,
                                msg.md.compressed_size as usize,
                                self.compress,
                            )?;
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
                        let reason = format!("Error committing load transaction ({}).", &err);
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

        #[cfg(test)]
        /// Compress a string using `snap`.
        ///
        /// # Arguments
        ///
        /// * `data` is the string that will be compressed.
        pub fn compress_str(data: &str) -> Result<Vec<u8>> {
            compress(data.as_bytes())
        }

        /// Compress a sequence of bytes.
        ///
        /// # Argument
        ///
        /// * `data` is the sequence of bytes that will be compressed.
        pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
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
        pub fn uncompress_str(compressed_data: &[u8]) -> Result<String> {
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
        pub fn uncompress(compressed_data: &[u8]) -> Result<Vec<u8>> {
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
                let testcase = include_str!("schema.sql");
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
        pub fn write(tx: &Transaction, history: &[u8], table: &str, column: &str, rid: i64) -> Result<()> {
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
        pub fn read(conn: &Connection, table: &str, column: &str, rid: i64) -> Result<Vec<u8>> {
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
                let testcase = include_str!("schema.sql");
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
