//! The hybrid database implementation.
use super::*;

pub(super) use v3::{data_adapter, load, reload};
mod v3 {
    //! The current implementation of a hybrid database
    use super::*;
    use crate::backend::filesys::WeatherHistory;

    /// Create the *hybrid* version of the data adapter.
    pub fn data_adapter(config: Config, weather_dir: WeatherDir) -> Result<Box<dyn DataAdapter>> {
        log::debug!("Hybrid data adapter.");
        Ok(Box::new(HybridDataAdapter { config, weather_dir }))
    }

    /// The *hybrid* data adapter implementation.
    #[derive(Debug)]
    struct HybridDataAdapter {
        config: Config,
        /// The weather data directory.
        weather_dir: WeatherDir,
    }
    impl DataAdapter for HybridDataAdapter {
        /// Get the hybrid data adapter configuration.
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
            // audit the histories to see which ones can be added
            let mut conn = db_conn!(&self.weather_dir)?;
            let (lid, histories) = metadata::examine_add_histories(&conn, daily_histories)?;
            // add metadata for the new archive histories
            let alias = &daily_histories.location.alias;
            let history_dates: Vec<NaiveDate> = histories.iter().map(|history| history.date).collect();
            let archive = WeatherArchive::open(alias, self.weather_dir.archive(alias))?;
            let tx = conn.transaction()?;
            for md in archive.iter_dates(history_dates, ArchiveMd::new)? {
                metadata::insert(&tx, lid, &md.date, md.compressed_size as usize, md.size as usize)?;
            }
            tx.commit()?;
            // report any differences adding to the archive and db
            let histories_added = cmp::max(file_histories.len(), histories.len());
            self.audit_add_histories(&daily_histories.histories, file_histories, histories);
            Ok(histories_added)
        }

        /// Returns the daily weather data history for a location.
        ///
        /// It is currently an error if more than 1 location is found through the location
        /// query.
        ///
        /// # Arguments
        ///
        /// * `location` is whose historical data will be found.
        /// * `history_range` specifies the date range that should be used.
        fn daily_histories(&self, location: Location, history_range: DateRange) -> Result<DailyHistories> {
            let file = self.weather_dir.archive(&location.alias);
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
            let db_sizes = query::db_size(&conn, metadata::TABLE_NAME)?;
            let history_counts = query::history_counts(&conn)?;
            let history_summaries = self
                .locations(criteria)?
                .into_iter()
                .map(|location| {
                    let count = history_counts.get(&location.alias);
                    let db_size = db_sizes.get(&location.alias);
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

    /// Loads the database based on the *hybrid* implementation of weather data.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `weather_dir` is the weather data directory.
    pub fn load(conn: &mut Connection, weather_dir: &WeatherDir) -> Result<()> {
        log::debug!("  metadata");
        let id_aliases = locations::id_aliases(conn)?;
        let tx = conn.transaction()?;
        for (lid, alias) in id_aliases {
            log::debug!("    {}", alias);
            let file = weather_dir.archive(&alias);
            let archive = WeatherArchive::open(&alias, file)?;
            for md in archive.iter_date_range(None, false, ArchiveMd::new)? {
                metadata::insert(&tx, lid, &md.date, md.compressed_size as usize, md.size as usize)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reload a locations weather history for the *hybrid* implementation of weather data.
    ///
    /// # Arguments
    ///
    /// * `conn` is the database connection that will be used.
    /// * `weather_dir` is the weather data directory.
    /// * `alias` is the location alias name.
    pub fn reload(conn: &mut Connection, weather_dir: &WeatherDir, alias: &str) -> Result<()> {
        let stopwatch = StopWatch::start_new();
        let lid = locations::location_id(conn, alias)?;
        let archive = WeatherArchive::open(&alias, weather_dir.archive(&alias))?;
        let tx = conn.transaction()?;
        metadata::delete(&tx, lid)?;
        for md in archive.iter_date_range(None, false, ArchiveMd::new)? {
            metadata::insert(&tx, lid, &md.date, md.compressed_size as usize, md.size as usize)?;
        }
        tx.commit()?;
        log::debug!("reload took {}", stopwatch);
        Ok(())
    }
}
