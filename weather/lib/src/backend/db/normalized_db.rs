//! The [DataAdapter] implementation using a normalized database schema.
use super::*;

pub(super) use v3::{data_adapter, loader::load, reload};
mod v3 {
    //! The [DataAdapter] implementation using a normalized database schema.
    use super::*;

    /// Create the *normalized* version of the data adapter.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory name.
    pub fn data_adapter(config: Config, weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
        log::debug!("Normalized data adapter");
        let data_adapter = NormalizedDataAdapter { config, weather_dir };
        Ok(Box::new(data_adapter))
    }

    /// The *normalized* data adapter implementation.
    // pub(crate) struct NormalizedDataAdapter {
    struct NormalizedDataAdapter {
        config: Config,
        /// The weather data directory.
        weather_dir: WeatherDir,
    }
    impl DataAdapter for NormalizedDataAdapter {
        /// Get the normalized data adapter configuration.
        fn config(&self) -> &Config {
            &self.config
        }
        /// Add weather data history for a location.
        ///
        /// # Arguments
        ///
        /// * `daily_histories` contains the location histories to add.
        fn add_daily_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
            // add histories to the archive first
            let file_histories = archive::add_histories(&self.weather_dir, daily_histories)?;
            // filter out histories that already exist in the db
            let mut conn = db_conn!(&self.weather_dir)?;
            let (lid, db_histories) = metadata::examine_add_histories(&conn, daily_histories)?;
            // get the archive metadata for histories that will be added to the database
            let alias = &daily_histories.location.alias;
            let archive = WeatherArchive::open(alias, self.weather_dir.archive(alias))?;
            let dates: Vec<NaiveDate> = db_histories.iter().map(|history| history.date).collect();
            let archive_md: Vec<ArchiveMd> = archive.iter_dates(dates, ArchiveMd::new)?.collect();
            // combine the histories and metadata
            let updates: Vec<(&History, &ArchiveMd)> =
                db_histories.iter().zip(archive_md.iter()).map(|(history, md)| (*history, md)).collect();
            // now add the histories.
            let size = size_estimate(&conn, "history")?;
            let mut tx = conn.transaction()?;
            for (history, md) in updates {
                let size = size
                    + history.description.as_ref().map_or(0, |s| s.len())
                    + history.precipitation_type.as_ref().map_or(0, |s| s.len());
                insert_history(&mut tx, lid, size, md.compressed_size as usize, history)?;
            }
            tx.commit()?;
            // report any differences adding to the archive and db
            let histories_added = cmp::max(file_histories.len(), db_histories.len());
            self.audit_add_histories(&daily_histories.histories, file_histories, db_histories);
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
            let daily_histories = query_history(&conn, &location.alias, date_range)?;
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
            let db_sizes = query::db_size(&conn, "history")?;
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
    fn insert_history(tx: &mut Transaction, lid: i64, size: usize, store_size: usize, history: &History) -> Result<()> {
        let mut data_stmt = tx.prepare_cached(INSERT_SQL)?;
        let mid = metadata::insert(tx, lid, &history.date, store_size, size)?;
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

    /// Reload a locations weather history for the *normalized* implementation of weather data.
    ///
    /// # Argument
    ///
    /// * `conn` is the database connection that will be used.
    /// * `weather_dir` is the weather data directory.
    /// * `alias` is the location that will be reloaded.
    pub fn reload(conn: &mut Connection, weather_dir: &WeatherDir, alias: &str) -> Result<()> {
        let stopwatch = StopWatch::start_new();
        let size = size_estimate(&conn, "history")?;
        let lid = locations::location_id(conn, alias)?;
        let archive = WeatherArchive::open(alias, weather_dir.archive(alias))?;
        const SQL: &str = r#"
        DELETE FROM history
        WHERE ROWID IN (
          SELECT h.ROWID FROM history AS h
          INNER JOIN metadata AS m ON h.mid = m.id
          WHERE m.lid = :lid
        )
        "#;
        let mut tx = conn.transaction()?;
        let mut stmt = tx.prepare(SQL)?;
        stmt.execute(named_params! {":lid": lid})?;
        drop(stmt);
        metadata::delete(&tx, lid)?;
        for (md, history) in archive.iter_date_range(None, false, archive_history_collector)? {
            insert_history(&mut tx, lid, size, md.compressed_size as usize, &history)?;
        }
        tx.commit()?;
        log::debug!("reload {} in {}", alias, stopwatch);
        Ok(())
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
        let mut rows = stmt.query(named_params! {":alias": alias, ":from": date_range.from, ":thru": date_range.to})?;
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

    pub mod loader {
        //! The normalized database archive loader for [History].
        use super::*;
        use archive::loader::*;
        use std::{
            sync::mpsc::{Receiver, Sender, TryRecvError},
            thread, time,
        };

        /// The data passed through the [ArchiveLoader].
        #[derive(Debug)]
        struct LoadMsg {
            /// The location table identifier.
            lid: i64,
            /// The archive metadata.
            md: ArchiveMd,
            /// The daily history.
            history: History,
        }

        /// Take the [History] archives and push them into the database.
        ///
        /// # Argument
        ///
        /// * `weather_dir` is the weather data directory.
        /// * `threads` is the number of workers to use getting data from archives.
        pub fn load(weather_dir: &WeatherDir, threads: usize) -> Result<()> {
            let conn = db_conn!(weather_dir)?;
            let size_estimate = size_estimate(&conn, "history")?;
            let archives = ArchiveQueue::new(&conn, weather_dir)?;
            let mut loader: ArchiveLoader<LoadMsg> = ArchiveLoader::new(threads);
            loader.execute(
                archives,
                || Box::new(HistoryProducer),
                || Box::new(HistoryConsumer { conn, base_size: size_estimate }),
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
            /// * `history` is the data that will be sent off to the consumer.
            /// * `sender` is used to pass data to the collector.
            fn send_history(&self, lid: i64, md: ArchiveMd, history: History, sender: &Sender<LoadMsg>) -> Result<()> {
                let msg = LoadMsg { lid, md, history };
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
                let mut history_count = 0;
                for (md, history) in archive.iter_date_range(None, false, archive_history_collector)? {
                    self.send_history(lid, md, history, sender)?;
                    history_count += 1;
                }
                Ok(history_count)
            }
        }

        /// The database history loader.
        struct HistoryConsumer {
            /// The database connection that will be used.
            conn: Connection,
            /// The base size of a row minus the text field lengths.
            base_size: usize,
        }
        impl ArchiveConsumer<LoadMsg> for HistoryConsumer {
            /// Called by the [ArchiveLoader] to collect the weather history being mined.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the weather data.
            fn collect(&mut self, receiver: Receiver<LoadMsg>) -> Result<usize> {
                let mut tx = self.conn.transaction()?;
                let mut count: usize = 0;
                // spin on the receiver until there's no one sending more data
                let pause = time::Duration::from_millis(1);
                loop {
                    match receiver.try_recv() {
                        Ok(msg) => {
                            let mut size = self.base_size + msg.history.description.as_ref().map_or(0, |s| s.len());
                            size += msg.history.precipitation_type.as_ref().map_or(Default::default(), |t| t.len());
                            insert_history(&mut tx, msg.lid, size, msg.md.compressed_size as usize, &msg.history)?;
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
}
