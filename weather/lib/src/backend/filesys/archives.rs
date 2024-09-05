//! Support for weather data saved in `ZIP` archives.
use super::*;

pub use v2::{
    history_collector as archive_history_collector, ArchiveMd, WeatherArchive, WeatherHistory, WeatherHistoryUpdate,
};
mod v2 {
    //! The current implementation to access weather data in ZIP archives.
    //!
    //! The implementation does not manage multi-client file access. That concern is left
    //! to the consumer of the module.
    use super::*;
    use crate::prelude::{DateRange, DateRanges, History, HistorySummary};
    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
    use std::{
        fs::{self, File, OpenOptions},
        io::{BufReader, Read, Write},
    };
    use toolslib::{fmt::commafy, stopwatch::StopWatch};
    use zip::{self, read::ZipFile, result::ZipError, write::FileOptions, DateTime, ZipArchive, ZipWriter};

    /// The [WeatherArchive] error builder.
    macro_rules! archive_err {
        ($id:expr, $reason:expr) => {
            Error::from(format!("WeatherArchive ({}): {}", $id, $reason))
        };
    }

    /// Create a description of the error if it is not [ZipError::FileNotFound].
    ///
    /// # Arguments
    ///
    /// *`err` is the [ZipError] whose description will be returned.
    fn ziperror_descr(err: ZipError) -> Option<String> {
        match err {
            ZipError::FileNotFound => None,
            _ => Some(err.to_string()),
        }
    }

    /// The definition of the `ZipArchive` reader.
    type ZipArchiveReader = ZipArchive<BufReader<File>>;

    /// The public view of a weather archive file.
    #[derive(Debug)]
    pub struct WeatherHistory(
        /// The managed weather archive.
        WeatherArchive,
    );
    impl WeatherHistory {
        /// Create a new instance of the weather archive manager.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location id.
        /// * `file` is the weather archive file.
        pub fn new(alias: &str, file: WeatherFile) -> Result<Self> {
            let archive = WeatherArchive::open(alias, file)?;
            Ok(Self(archive))
        }
        /// Creates a summary of the weather history statistics.
        pub fn summary(&self) -> Result<HistorySummary> {
            let mut files: usize = 0;
            let mut size: u64 = 0;
            let mut compressed_size: u64 = 0;
            let iter = self.0.iter_date_range(None, false, ArchiveMd::new)?;
            iter.for_each(|md| {
                files += 1;
                size += md.size;
                compressed_size += md.compressed_size;
            });
            Ok(HistorySummary {
                location_id: self.0.alias.to_string(),
                count: files,
                overall_size: Some(self.0.file.size() as usize),
                raw_size: Some(size as usize),
                compressed_size: Some(compressed_size as usize),
            })
        }
        /// Get the weather history dates that are available.
        pub fn dates(&self) -> Result<DateRanges> {
            let stopwatch = StopWatch::start_new();
            let iter = self.0.iter_date_range(None, false, ArchiveMd::new)?;
            let dates: Vec<NaiveDate> = iter.map(|md| md.date).collect();
            log::trace!("WeatherHistory: collect dates {}", &stopwatch);
            let date_ranges = DateRange::from_dates(dates);
            Ok(DateRanges { location_id: self.0.alias.to_string(), date_ranges })
        }
        /// Get an iterator of daily weather history for a location.
        ///
        /// # Arguments
        ///
        /// * `filter` restricts the range of the historical weather data.
        ///
        pub fn daily_histories(&self, filter: &DateRange) -> Result<Vec<History>> {
            let iter = self.0.iter_date_range(Some(filter), true, history_decoder)?;
            let histories = iter.collect();
            Ok(histories)
        }
    }

    /// The weather archive file updater.
    #[derive(Debug)]
    pub struct WeatherHistoryUpdate(
        /// The weather archive that will be updated.
        WeatherArchive,
    );
    impl WeatherHistoryUpdate {
        /// Create a new instance of the weather history updater.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location id.
        /// * `file` is the weather archive file.
        pub fn new(alias: &str, file: WeatherFile) -> Result<Self> {
            let archive = WeatherArchive::open(alias, file)?;
            Ok(Self(archive))
        }
        /// Add histories to the weather archive that don't already exist.
        ///
        /// # Arguments
        ///
        /// * `histories` are the histories that will be added.
        pub fn add(&mut self, histories: &Vec<History>) -> Result<Vec<NaiveDate>> {
            // find histories dates that already exist
            let mut stopwatch = StopWatch::start_new();
            let mut already_exists: Vec<NaiveDate> = Vec::with_capacity(histories.len());
            for md in self.0.iter_date_range(None, true, ArchiveMd::new)? {
                if histories.iter().any(|history| history.date == md.date) {
                    already_exists.push(md.date);
                    // you're done if all the histories to add exist
                    if already_exists.len() == histories.len() {
                        break;
                    }
                }
            }
            // filter out the histories that already exist
            let okay_to_add: Vec<&History> = histories
                .iter()
                .filter_map(|history| match already_exists.iter().any(|date| history.date == *date) {
                    true => None,
                    false => Some(history),
                })
                .collect();
            log::trace!("collect additions {}", &stopwatch);
            // now add the histories that weren't found to the archive
            stopwatch.start();
            let dates_added: Vec<NaiveDate> = okay_to_add.iter().map(|h| h.date).collect();
            if !okay_to_add.is_empty() {
                let mut writer = self.0.archive_writer();
                writer.write(okay_to_add)?;
            }
            stopwatch.stop();
            if !already_exists.is_empty() {
                let dates = already_exists.iter().map(|date| date.to_string()).collect::<Vec<String>>().join(", ");
                log::info!("Location '{}': these histories already exist {}.", self.0.alias, dates);
            }
            log::trace!("archive update added {} in {}", dates_added.len(), &stopwatch);
            Ok(dates_added)
        }
    }

    /// The manager for a `Zip` archive with weather data.
    #[derive(Debug)]
    pub struct WeatherArchive {
        /// The unique identifier for a location.
        pub(in crate::backend) alias: String,
        /// The file that contains weather data.
        file: WeatherFile,
    }
    impl WeatherArchive {
        /// Create the manager for an existing weather data archive.
        ///
        /// An error will be returned if the archive does not exist or is not valid.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        /// * `file` is the archive containing of weather data.
        pub fn open(alias: &str, mut file: WeatherFile) -> Result<Self> {
            let stopwatch = StopWatch::start_new();
            file.refresh();
            let result = if !file.exists() {
                Err(archive_err!(alias, format!("'{}' does not exist...", &file)))
            } else {
                // use a buffer reader here otherwise opening the archive is really slow....
                let reader = BufReader::new(file.reader()?);
                match ZipArchive::new(reader) {
                    // unfortunately you have to drop the zip archive which makes open/create expensive
                    Ok(_) => Ok(Self { alias: alias.to_string(), file }),
                    Err(error) => Err(archive_err!(alias, &error)),
                }
            };
            log::trace!("WeatherArchive: open {} {}us", alias, commafy(stopwatch.elapsed().as_micros()));
            result
        }
        /// Creates a new weather data archive and the manager for it
        ///
        /// An error will be returned if the archive exists or there are problems trying to create it.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        /// * `file` is the container of weather data.
        pub fn create(alias: &str, mut file: WeatherFile) -> Result<Self> {
            file.refresh();
            if file.exists() {
                Err(archive_err!(&alias, format!("'{}' already exists...", &file)))
            } else {
                // touch the file so the writer can be returned.
                if let Err(err) = OpenOptions::new().create(true).write(true).open(&file.to_string()) {
                    Err(archive_err!(alias, &format!("Error creating archive file ({}), {}", &file, &err)))
                } else {
                    let writer = file.writer()?;
                    let mut archive = ZipWriter::new(writer);
                    match archive.finish() {
                        Ok(_) => Self::open(alias, file),
                        Err(err) => Err(archive_err!(alias, &err)),
                    }
                }
            }
        }
        /// Creates an archive iterator that returns weather data history.
        ///
        /// All history in the archive is returned if `filter` is `None`.
        ///
        /// # Arguments
        ///
        /// * `filter` restricts history data to a range of dates.
        /// * `sort` when true will order history by ascending date.
        /// * `builder` is called by the iterator to create the history data.
        pub fn iter_date_range<T>(
            &self,
            filter: Option<&DateRange>,
            sort: bool,
            builder: HistoryBuilder<T>,
        ) -> Result<ArchiveIter<T>> {
            let inner = self.file.reader()?;
            match ZipArchive::new(BufReader::new(inner)) {
                Ok(mut reader) => {
                    let mut history_dates = self.filter_history(&mut reader, filter);
                    if sort {
                        history_dates.sort()
                    }
                    Ok(ArchiveIter::new(&self.alias, reader, history_dates, builder))
                }
                Err(err) => Err(archive_err!(&self.alias, &format!("get_reader error ({}).", &err))),
            }
        }

        /// Creates an archive iterator that returns weather data history for a collection of dates.
        ///
        /// # Arguments
        ///
        /// * `filter` restricts history data to these dates.
        /// * `builder` is called by the iterator to create the history data.
        pub fn iter_dates<T>(&self, filter: Vec<NaiveDate>, builder: HistoryBuilder<T>) -> Result<ArchiveIter<T>> {
            let inner = self.file.reader()?;
            match ZipArchive::new(BufReader::new(inner)) {
                Ok(mut reader) => {
                    let history_dates: Vec<NaiveDate> = filter
                        .iter()
                        .filter_map(|date| {
                            let name = Self::date_to_filename(&self.alias, date);
                            match reader.by_name(name.as_str()) {
                                Ok(_) => Some(*date),
                                Err(err) => {
                                    if let Some(descr) = ziperror_descr(err) {
                                        let reason = format!("ZipArchive::by_name() error ({}).", descr);
                                        log::error!("{}", archive_err!(self.alias, reason));
                                    }
                                    None
                                }
                            }
                        })
                        .collect();
                    Ok(ArchiveIter::new(&self.alias, reader, history_dates, builder))
                }
                Err(err) => {
                    let reason = format!("ZipArchive error ({})", err);
                    Err(archive_err!(&self.alias, reason))
                }
            }
        }
        /// Get the [ArchiveWriter].
        pub fn archive_writer(&mut self) -> ArchiveWriter {
            ArchiveWriter::new(self)
        }
        /// Get the weather history dates in the archive.
        ///
        /// If `filter` is provided, only dates covered by the history range are returned.
        /// The returned date collection is not guaranteed to be ordered.
        ///
        /// # Arguments
        ///
        /// * `reader` is used to get the history dates.
        /// * `filter` is used to restrict the dates that will be returned.
        fn filter_history(&self, reader: &mut ZipArchiveReader, filter: Option<&DateRange>) -> Vec<NaiveDate> {
            let stopwatch = StopWatch::start_new();
            let dates = reader
                .file_names()
                .filter_map(|filename| match WeatherArchive::filename_to_date(filename) {
                    Ok(date) => Some(date),
                    Err(err) => {
                        let reason = format!("filter_history ({}).", err);
                        log::error!("{}", archive_err!(self.alias, reason));
                        None
                    }
                })
                .filter(|date| match filter {
                    Some(range) => range.covers(date),
                    None => true,
                })
                .collect();
            log::trace!("WeatherArchive: filter_history {}us", commafy(stopwatch.elapsed().as_micros()));
            dates
        }
        /// Build the internal archive filename to the provided date.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location id.
        /// * `date` is the history date that will be embedded into the filename.
        fn date_to_filename(alias: &str, date: &NaiveDate) -> String {
            format!("{}/{}-{}.json", alias, alias, toolslib::date_time::fmt_date(date, "%Y%m%d"))
        }
        /// Extracts the date from internal archive filename.
        ///
        /// An error is returned if the filename is not a valid history name.
        ///
        /// # Arguments
        ///
        /// * `history_name` is a weather archive filename containing the embedded date.
        fn filename_to_date(history_name: &str) -> Result<NaiveDate> {
            let ymd_offset = "yyyymmdd.json".len();
            if ymd_offset > history_name.len() {
                Err(Error::from(format!("malformed history name ({}).", history_name)))
            } else {
                let ymd_index = history_name.len() - ymd_offset;
                let ymd: &str = &history_name[ymd_index..ymd_index + 8];
                if !ymd.chars().all(char::is_numeric) {
                    Err(Error::from(format!("history date not found ({}).", history_name)))
                } else {
                    let year = ymd[..4].parse().unwrap();
                    let month = ymd[4..6].parse().unwrap();
                    let day = ymd[6..].parse().unwrap();
                    match NaiveDate::from_ymd_opt(year, month, day) {
                        Some(date) => Ok(date),
                        None => Err(Error::from(format!("illegal history date ({}).", history_name))),
                    }
                }
            }
        }
    }

    /// A bean providing metrics about a weather history file in the archive.
    #[derive(Debug)]
    pub struct ArchiveMd {
        /// The location identifier.
        #[allow(unused)]
        pub alias: String,
        /// The date associated with the history file in the archive.
        pub date: NaiveDate,
        /// The size of the file in the archive.
        pub compressed_size: u64,
        /// The actual size of the file.
        pub size: u64,
        /// The last modified timestamp of the history file in the archive.
        #[allow(unused)]
        pub mtime: i64,
    }
    impl ArchiveMd {
        /// Create a new instance of the weather history file metrics in the archive.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        /// * `date` is the date associated with the history file.
        /// * `zipfile` provides access to the history file metrics.
        pub fn new(alias: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<Self> {
            let mtime = Self::datetime_to_millis(alias, zipfile.last_modified());
            Ok(Self {
                alias: alias.to_string(),
                date: date.clone(),
                compressed_size: zipfile.compressed_size(),
                size: zipfile.size(),
                mtime,
            })
        }
        /// Convert the `ZIP` date time to milliseconds.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        /// * `datetime` is the `ZIP` file timestamp.
        pub fn datetime_to_millis(alias: &str, datetime: DateTime) -> i64 {
            let default = DateTime::default();
            if datetime.datepart() == default.datepart() && datetime.timepart() == default.timepart() {
                0
            } else {
                let year = datetime.year() as i32;
                let month = datetime.month() as u32;
                let day = datetime.day() as u32;
                if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                    let hour = datetime.hour() as u32;
                    let minute = datetime.minute() as u32;
                    let second = datetime.second() as u32;
                    if let Some(time) = NaiveTime::from_hms_opt(hour, minute, second) {
                        let datetime = NaiveDateTime::new(date, time);
                        datetime.timestamp_millis()
                    } else {
                        let reason = format!("NaiveTime error for HHMMSS ({:02}{:02}{:02})", hour, minute, second);
                        log::error!("{}", &archive_err!(alias, reason));
                        0
                    }
                } else {
                    let reason = format!("NaiveDate error for YYYYMMDD ({:04}{:02}{:02})", year, month, day);
                    log::error!("{}", &archive_err!(alias, reason));
                    0
                }
            }
        }
    }

    /// The function signature used by the weather archive iterator to create history data.
    type HistoryBuilder<T> = fn(&str, &NaiveDate, ZipFile) -> Result<T>;

    /// The [HistoryBuilder] used to get [History] from the archive.
    ///
    /// # Arguments
    ///
    /// * `alias` is the location alias name.
    /// * `date` is the weather history date.
    /// * `zipfile` is the archive weather history file.
    pub fn history_decoder(alias: &str, date: &NaiveDate, mut zipfile: ZipFile) -> Result<History> {
        let size = zipfile.size() as usize;
        let mut data: Vec<u8> = Vec::with_capacity(size);
        match zipfile.read_to_end(&mut data) {
            Ok(_) => history::from_bytes(alias, &data),
            Err(err) => {
                let reason = format!("error reading {} history ({})", date, err);
                Err(archive_err!(alias, reason))
            }
        }
    }

    /// The [HistoryBuilder] used to collect history from the archive.
    ///
    /// # Arguments
    ///
    /// * `alias` is the location alias name.
    /// * `date` is the weather history date.
    /// * `zipfile` is the archive weather history file.
    pub fn history_collector(alias: &str, date: &NaiveDate, mut zipfile: ZipFile) -> Result<(ArchiveMd, History)> {
        let size = zipfile.size() as usize;
        let mut data: Vec<u8> = Vec::with_capacity(size);
        match zipfile.read_to_end(&mut data) {
            Ok(_) => {
                let history = history::from_bytes(alias, &data)?;
                let md = ArchiveMd::new(alias, date, zipfile)?;
                Ok((md, history))
            }
            Err(err) => {
                let reason = format!("error reading {} history ({})", date, err);
                Err(archive_err!(alias, reason))
            }
        }
    }

    /// The low-level iterator over weather history in the archive.
    pub struct ArchiveIter<T> {
        /// The location identifier.
        alias: String,
        /// The `ZIP` archive reader.
        reader: ZipArchiveReader,
        /// The weather history dates that will be returned.
        dates: Vec<NaiveDate>,
        /// The index of the next weather history date.
        index: usize,
        /// The function used to create the appropriate type of history data.
        history_builder: HistoryBuilder<T>,
    }
    impl<T> ArchiveIter<T> {
        /// Create a new instance of the iterator.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location identifier.
        /// * `reader` is the `ZIP` archive reader.
        /// * `dates` identify what weather history will be returned.
        /// * `make` is what the iterator uses to return the history data.
        fn new(alias: &str, reader: ZipArchiveReader, dates: Vec<NaiveDate>, make: HistoryBuilder<T>) -> Self {
            Self { alias: alias.to_string(), reader, dates, index: 0, history_builder: make }
        }
    }
    impl<T> Iterator for ArchiveIter<T> {
        type Item = T;
        /// Create the next weather data history instance.
        fn next(&mut self) -> Option<Self::Item> {
            let mut history = None;
            if self.index < self.dates.len() {
                let date = self.dates[self.index];
                self.index += 1;
                let history_name = WeatherArchive::date_to_filename(&self.alias, &date);
                match self.reader.by_name(&history_name) {
                    Ok(zip_file) => match (self.history_builder)(&self.alias, &date, zip_file) {
                        Ok(data) => {
                            history.replace(data);
                        }
                        Err(err) => {
                            let reason = format!("HistoryBuilder ({}).", err);
                            log::error!("{}", archive_err!(self.alias, reason));
                        }
                    },
                    Err(err) => {
                        if let Some(descr) = ziperror_descr(err) {
                            let reason = format!("ZipArchive::by_name() ({}).", descr);
                            log::error!("{}", archive_err!(self.alias, reason));
                        }
                    }
                }
            }
            history
        }
    }

    /// The manager that adds weather history to an archive.
    #[derive(Debug)]
    pub struct ArchiveWriter<'a> {
        /// The archive that will be updated.
        archive: &'a WeatherArchive,
        /// The pathname of the archive that will actually have data added to it.
        writable: PathBuf,
    }
    impl<'a> ArchiveWriter<'a> {
        /// The extension that identifies a writable archive.
        const UPDATE_EXT: &'static str = "upd";
        /// The extension that identifies an archive backup.
        const BACKUP_EXT: &'static str = "bu";
        /// Create a new instance of the archive writer.
        ///
        /// # Arguments
        ///
        /// `archive` is what will be updated with new history.
        fn new(archive: &'a WeatherArchive) -> Self {
            let writable = archive.file.path().with_extension(Self::UPDATE_EXT);
            Self { archive, writable }
        }
        /// Adds history to the archive.
        ///
        /// # Arguments
        ///
        /// `histories` is what will be added to the archive.
        pub fn write(&mut self, histories: Vec<&History>) -> Result<()> {
            let mut writer = self.open()?;
            for history in histories {
                let data = history::to_bytes(history)?;
                self.write_history(&mut writer, &history.date, &data[..])?;
            }
            self.close(writer)
        }
        /// Writes history into the archive.
        ///
        /// # Arguments
        ///
        /// * `writer` will be used to add the history.
        /// * `date` is the data associated with the history.
        /// * `data` is the history serialized into a sequence of bytes.
        fn write_history(&self, writer: &mut ZipWriter<File>, date: &NaiveDate, data: &[u8]) -> Result<()> {
            let now = Utc::now().naive_utc();
            let mtime = DateTime::from_date_and_time(
                now.year() as u16,
                now.month() as u8,
                now.day() as u8,
                now.hour() as u8,
                now.minute() as u8,
                now.second() as u8,
            )
            .unwrap();
            let filename = WeatherArchive::date_to_filename(&self.archive.alias, date);
            let options =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated).last_modified_time(mtime);
            if let Err(err) = writer.start_file(filename, options) {
                let reason = format!("{} start_file error ({}).", date, &err);
                Err(archive_err!(&self.archive.alias, reason))
            } else if let Err(err) = writer.write_all(data) {
                let reason = format!("{} write_all err ({}).", date, &err);
                Err(archive_err!(&self.archive.alias, reason))
            } else {
                Ok(())
            }
        }
        /// Creates the [ZipWriter] that will update the archive.
        ///
        /// In order to add data the archive is first copied to the writable path. When done adding history the
        /// archive will be restored when the [ZipWriter] is closed.
        fn open(&self) -> Result<ZipWriter<File>> {
            self.copy(self.archive.file.path(), &self.writable)?;
            match File::options().read(true).write(true).open(&self.writable) {
                Ok(file) => match ZipWriter::new_append(file) {
                    Ok(zip_writer) => Ok(zip_writer),
                    Err(err) => {
                        let reason = format!("'{}' zip writer error ({}).", self.archive.file.filename, err);
                        Err(archive_err!(&self.archive.alias, reason))
                    }
                },
                Err(err) => {
                    let reason = format!("error open writable archive ({}).", &err);
                    Err(archive_err!(&self.archive.alias, reason))
                }
            }
        }
        /// Close the [ZipWriter] and restore the archive.
        ///
        /// When the archive is opened a copy is made and a [ZipWriter] returned that will be used. After it
        /// is closed, the updated archive replaces the original.
        ///
        /// # Arguments
        ///
        /// * `writer` is what was used to update the archive histories.
        fn close(&self, writer: ZipWriter<File>) -> Result<()> {
            drop(writer);
            // try to safely replace the updated archive
            let backup = self.archive.file.path().with_extension(Self::BACKUP_EXT);
            // backup the archive
            match self.copy(self.archive.file.path(), &backup) {
                // replace the archive with the one that was updated
                Ok(_) => match fs::rename(&self.writable, &self.archive.file.path()) {
                    Ok(_) => {
                        // remove the backed up file
                        match fs::remove_file(&backup) {
                            Ok(_) => (),
                            Err(err) => {
                                log::warn!("error removing archive backup {} ({})", backup.display(), err);
                            }
                        };
                        Ok(())
                    }
                    Err(err) => {
                        // try to restore the original archive
                        match fs::rename(&backup, self.archive.file.path()) {
                            Ok(_) => log::info!("{}: original archive restored.", self.archive.alias),
                            Err(err) => {
                                let reason = format!("could not restore backup ({}).", err);
                                log::error!("{}", archive_err!(self.archive.alias, reason));
                            }
                        };
                        let reason = format!("error replacing updated archive ({})", err);
                        Err(archive_err!(&self.archive.alias, reason))
                    }
                },
                Err(err) => {
                    let reason = format!("error creating archive backup {} ({})", backup.display(), err);
                    Err(archive_err!(&self.archive.alias, reason))
                }
            }
        }
        fn copy(&self, from: &Path, to: &Path) -> Result<()> {
            match fs::copy(from, to) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        // before running on Unix you need to set check/set file permissions
                    }
                    Ok(())
                }
                Err(err) => {
                    let from_name = from.file_name().unwrap().to_str().unwrap();
                    let to_name = to.file_name().unwrap().to_str().unwrap();
                    let reason = format!("error copying {} to {} ({})", from_name, to_name, err);
                    Err(archive_err!(&self.archive.alias, reason))
                }
            }
        }
    }
    impl<'a> Drop for ArchiveWriter<'a> {
        /// If something bad happens adding history, this attempts to clean up files.rs that might be
        /// left hanging around.
        fn drop(&mut self) {
            // do your best to clean up
            if self.writable.exists() {
                match fs::remove_file(&self.writable) {
                    Ok(_) => (),
                    Err(err) => {
                        log::warn!("Drop: error deleting {} ({}).", self.writable.display(), err);
                    }
                }
            }
            let backup = self.archive.file.path().with_extension(Self::BACKUP_EXT);
            if backup.exists() {
                match fs::remove_file(&backup) {
                    Ok(_) => (),
                    Err(err) => {
                        log::warn!("Drop: error deleting {} ({}).", backup.display(), err);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use toolslib::date_time::get_date;

        #[test]
        fn create_open() {
            let fixture = testlib::TestFixture::create();
            let weather_dir = WeatherDir::try_from(fixture.to_string()).unwrap();
            let alias = "init";
            macro_rules! get_file {
                () => {
                    // weather_dir.get_file(&test_archive!(alias))
                    weather_dir.archive(alias)
                };
            }
            assert!(!get_file!().exists());
            assert!(WeatherArchive::open(alias, get_file!()).is_err());
            assert!(WeatherArchive::create(alias, get_file!()).is_ok());
            assert!(get_file!().exists());
            assert!(WeatherArchive::open(alias, get_file!()).is_ok());
            assert!(WeatherArchive::create(alias, get_file!()).is_err());
        }

        #[test]
        fn weather_data_iterator() {
            // don't copy files.rs use the test resources... just don't update files.rs!!!
            let resources = testlib::test_resources().join("filesys");
            let weather_dir = WeatherDir::new(resources).unwrap();
            let alias = "testmd";
            // verify the test data
            let file = weather_dir.archive(alias);
            assert!(file.exists());
            // set up the testcase
            let from = get_date(2014, 4, 1);
            let to = get_date(2014, 4, 5);
            let history_range = DateRange::new(from, to);
            let archive = WeatherArchive::open(alias, file).unwrap();
            let mut testcase = archive.iter_date_range(Some(&history_range), true, ArchiveMd::new).unwrap();
            let mut date_validator = history_range.into_iter();
            assert_eq!(testcase.next().unwrap().date, date_validator.next().unwrap());
            assert_eq!(testcase.next().unwrap().date, date_validator.next().unwrap());
            assert_eq!(testcase.next().unwrap().date, date_validator.next().unwrap());
            assert_eq!(testcase.next().unwrap().date, date_validator.next().unwrap());
            assert_eq!(testcase.next().unwrap().date, date_validator.next().unwrap());
            assert!(testcase.next().is_none());
        }

        #[test]
        fn history_name() {
            let date = get_date(2023, 7, 5);
            let history_name = WeatherArchive::date_to_filename("testcase", &date);
            let history_date = WeatherArchive::filename_to_date(&history_name).unwrap();
            assert_eq!(history_date, date);
            assert!(WeatherArchive::filename_to_date("20230705.json").is_ok());
            assert!(WeatherArchive::filename_to_date("a0230705.json").is_err());
            assert!(WeatherArchive::filename_to_date("2023b705.json").is_err());
            assert!(WeatherArchive::filename_to_date("202307c5.json").is_err());
            assert!(WeatherArchive::filename_to_date("20230705.json2").is_err());
        }

        #[test]
        fn history_summary() {
            let alias = "testmd";
            let resources = testlib::test_resources().join("filesys");
            let weather_dir = WeatherDir::new(resources).unwrap();
            let file = weather_dir.archive(alias);
            let weather_history = WeatherHistory::new(alias, file).unwrap();
            let testcase = weather_history.summary().unwrap();
            assert_eq!(testcase.location_id, alias);
            assert_eq!(testcase.count, 28);
            assert_eq!(testcase.overall_size, Some(43172));
            assert_eq!(testcase.raw_size, Some(263500));
            assert_eq!(testcase.compressed_size, Some(39510));
        }

        #[test]
        fn writer() {
            // set up the testcase
            let fixture = testlib::TestFixture::create();
            let weather_path = PathBuf::from(&fixture);
            let weather_dir = WeatherDir::new(weather_path.clone()).unwrap();
            let archive_file = weather_dir.archive("test");
            let mut archive = WeatherArchive::create("test", archive_file).unwrap();
            let original_archive_len = archive.file.size();
            let archive_writer = ArchiveWriter::new(&archive);
            // spot check opening
            let mut zip_writer = archive_writer.open().unwrap();
            let update_file = archive.file.path().with_extension(ArchiveWriter::UPDATE_EXT);
            assert!(archive.file.exists());
            assert!(update_file.exists());
            // spot check writing to the archive
            let history_data = "Content doesn't matter to the writer...";
            let date = NaiveDate::from_ymd_opt(2023, 9, 20).unwrap();
            archive_writer.write_history(&mut zip_writer, &date, history_data.as_bytes()).unwrap();
            // spot check closing
            archive_writer.close(zip_writer).unwrap();
            assert!(!update_file.exists());
            assert!(!archive.file.path().with_extension(ArchiveWriter::BACKUP_EXT).exists());
            drop(archive_writer);
            archive.file.refresh();
            assert_ne!(original_archive_len, archive.file.size());
            let mut iter = archive.iter_date_range(None, false, ArchiveMd::new).unwrap();
            let md = iter.next().unwrap();
            assert_eq!(md.alias, "test");
            assert_eq!(md.date, date);
            assert!(iter.next().is_none());
        }

        #[allow(unused)]
        // of course this is hard coded to my workstation
        const SOURCE_WEATHER_DATA: &str = r"C:\Users\rncru\dev\weather_data";
    }
}
