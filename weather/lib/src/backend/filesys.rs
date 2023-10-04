//! The filesystem objects that support implementing weather data using `ZIP` archives.

pub(crate) use v1::{archive_adapter, weather_dir, WeatherDir};
pub(in crate::backend) use v1::{
    weather_locations, ArchiveData, ArchiveMd, WeatherArchive, WeatherFile, WeatherHistory, WeatherHistoryUpdate,
};

mod v1 {
    //! The first generation of the new file based weather data implmentation
    #[cfg(test)]
    use crate::backend::testlib;

    use crate::backend::{history, Error, Result};
    use crate::prelude::{DateRange, DateRanges, History, HistorySummary, Location};
    use std::{
        fmt::Display,
        fs::File,
        path::{Path, PathBuf},
    };

    /// Get a [WeatherDir] instance.
    pub(crate) fn weather_dir(dirname: &str) -> Result<WeatherDir> {
        let weather_dir = if dirname.len() > 0 {
            WeatherDir::try_from(dirname)?
        } else if let Ok(env_pathname) = std::env::var("WEATHER_DATA") {
            WeatherDir::try_from(env_pathname)?
        } else {
            WeatherDir::try_from("weather_data")?
        };
        Ok(weather_dir)
    }

    // expose the file utilities to the module
    pub(crate) use file::{WeatherDir, WeatherFile};
    mod file {
        //! Support for filesystem access.
        //!
        use super::*;
        use std::{
            fs::{Metadata, OpenOptions},
            io::ErrorKind,
        };

        /// The [WeatherDir] error builder.
        macro_rules! dir_err {
            ($id:expr, $reason:expr) => {
                Error::from(format!("WeatherDir ({}): {}", $id, $reason))
            };
        }

        /// The manager responsible for stat, readers, and writers to file contents in the weather directory
        #[derive(Debug)]
        pub(crate) struct WeatherDir(
            /// The directory managed by the weather directory.
            PathBuf,
        );
        impl Display for WeatherDir {
            /// Use this trait to expose the weather directory pathname.
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0.as_path().display())
            }
        }
        impl TryFrom<String> for WeatherDir {
            type Error = Error;
            /// Create a [WeatherDir] instance using the string as a directory pathname.
            fn try_from(dirname: String) -> std::result::Result<Self, Self::Error> {
                WeatherDir::new(PathBuf::from(dirname))
            }
        }
        impl TryFrom<&str> for WeatherDir {
            type Error = Error;
            /// Create a [WeatherDir] instance using the string as a directory pathname.
            fn try_from(dirname: &str) -> std::result::Result<Self, Self::Error> {
                WeatherDir::new(PathBuf::from(dirname))
            }
        }
        impl WeatherDir {
            /// Creates a new instance of the weather directory manager.
            ///
            /// An error will be returned if the directory does not exist or it does exist but is not a directory.
            ///
            /// # Arguments
            ///
            /// * `directory_name` is the name of the directory.
            pub(crate) fn new(path: PathBuf) -> Result<WeatherDir> {
                match path.is_dir() {
                    true => Ok(WeatherDir(path)),
                    false => Err(dir_err!(path.display().to_string(), "Not a directory...")),
                }
            }
            /// Get a weather file from within the managed directory.
            ///
            /// # Arguments
            ///
            /// * `filename` is the name of the file within the weather directory.
            pub fn file(&self, filename: &str) -> WeatherFile {
                WeatherFile::new(self.0.join(filename))
            }
            pub fn archive(&self, alias: &str) -> WeatherFile {
                let archive_name = self.0.join(alias).with_extension("zip");
                WeatherFile::new(archive_name)
            }
            /// Get the weather directory path.
            pub fn path(&self) -> &Path {
                self.0.as_path()
            }
        }

        /// The [WeatherFile] error builder.
        macro_rules! file_err {
            ($id:expr, $reason:expr) => {
                Error::from(format!("WeatherFile {}: {}", $id, $reason))
            };
        }

        /// The manager of a file within the weather directory.
        #[derive(Debug)]
        pub(crate) struct WeatherFile {
            /// The file name within the weather directory.
            pub filename: String,
            /// The files path.
            path: PathBuf,
            /// The filesystem metadata for the file.
            fs_metadata: Option<Metadata>,
        }
        impl Display for WeatherFile {
            /// Use the trait to get the pathname of the file.
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.path.display())
            }
        }
        impl WeatherFile {
            /// Create the manager for files in the weather directory.
            ///
            /// # Arguments
            ///
            /// * `path` is the weather data file returned by the [`WeatherDir`].
            fn new(path: PathBuf) -> Self {
                // this should always work since the path comes from a DirEntry
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let fs_metadata = match path.metadata() {
                    Ok(metadata) => Some(metadata),
                    Err(err) => {
                        if err.kind() != ErrorKind::NotFound {
                            log::error!("{}", &file_err!(filename, &err));
                        }
                        None
                    }
                };
                WeatherFile { filename, path, fs_metadata }
            }
            /// Refresh the filesystem metadata.
            pub(in crate::backend) fn refresh(&mut self) {
                match self.path.metadata() {
                    Ok(metadata) => self.fs_metadata.replace(metadata),
                    Err(err) => {
                        if err.kind() != ErrorKind::NotFound {
                            log::error!("{}", file_err!(&self.filename, &err));
                        }
                        self.fs_metadata.take()
                    }
                };
            }
            /// Indicates if the file exists or does not.
            pub(in crate::backend) fn exists(&self) -> bool {
                self.fs_metadata.is_some()
            }
            /// Get the size of the file.
            pub(in crate::backend) fn size(&self) -> u64 {
                match &self.fs_metadata {
                    Some(md) => md.len(),
                    None => 0,
                }
            }
            /// Get the writer that can be used to update a Zip archive.
            pub(in crate::backend) fn writer(&self) -> Result<File> {
                match File::options().read(true).write(true).open(&self.path) {
                    Ok(file) => Ok(file),
                    Err(err) => Err(file_err!(&self.filename, &format!("open read/write error ({}).", &err))),
                }
            }
            /// Get the reader that can be used to read the contents of an Zip archive.
            pub(in crate::backend) fn reader(&self) -> Result<File> {
                match OpenOptions::new().read(true).open(&self.path) {
                    Ok(file) => Ok(file),
                    Err(err) => Err(file_err!(&self.filename, &format!("open read error ({})...", &err))),
                }
            }
            pub(in crate::backend) fn path(&self) -> &Path {
                self.path.as_path()
            }
        }

        #[cfg(test)]
        mod test {
            use super::*;
            use std::io::{Read, Write};

            #[test]
            fn weather_file() {
                let fixture = testlib::TestFixture::create();
                let filename = "testfile.dat";
                let mut testcase = WeatherFile::new(PathBuf::from(&fixture).join(filename));
                // verify metadata for a file that does not exist
                assert_eq!(testcase.filename, filename);
                assert!(!testcase.exists());
                assert_eq!(testcase.size(), 0);
                // create the file and content
                let content = "testcase file content...";
                {
                    OpenOptions::new().create(true).write(true).open(&testcase.to_string()).unwrap();
                    testcase.writer().unwrap().write_all(content.as_bytes()).unwrap();
                }
                // verify metadata behaviour
                assert!(!testcase.exists());
                assert_eq!(testcase.size(), 0);
                testcase.refresh();
                assert_eq!(testcase.filename, filename);
                assert!(testcase.exists());
                assert_eq!(testcase.size(), content.len() as u64);
                // verify reading the file content
                {
                    let mut reader = testcase.reader().unwrap();
                    let mut file_content = String::new();
                    reader.read_to_string(&mut file_content).unwrap();
                    assert_eq!(&file_content, content);
                }
            }

            #[test]
            fn weather_dir() {
                // setup the test case
                let fixture = testlib::TestFixture::create();
                let filename = "locations.json";
                let resource = testlib::test_resources().join("filesys").join(filename);
                fixture.copy_resources(&resource);
                // now spot check it
                let testcase = WeatherDir::try_from(fixture.to_string()).unwrap();
                let file = testcase.file(filename);
                assert!(file.exists());
                assert_eq!(file.size(), 515);
            }
        }
    }

    pub(in crate::backend) use archive::{
        ArchiveData, ArchiveMd, WeatherArchive, WeatherHistory, WeatherHistoryUpdate,
    };
    mod archive {
        //! Support for weather data saved in `ZIP` archives.
        //!
        //! The implementation does not manage multi-client file access. That concern is left
        //! to the consummer of the module.
        use super::*;
        use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
        use serde_json as json;
        use std::{
            collections::HashSet,
            fs::{self, OpenOptions},
            io::{BufReader, Read, Write},
        };
        use toolslib::{date_time::isodate, fmt::commafy, stopwatch::StopWatch};
        use zip::{self, read::ZipFile, write::FileOptions, DateTime, ZipArchive, ZipWriter};

        /// The [WeatherArchive] error builder.
        macro_rules! archive_err {
            ($id:expr, $reason:expr) => {
                Error::from(format!("WeatherArchive ({}): {}", $id, $reason))
            };
        }

        /// The definition of the `ZipArchive` reader.
        type ZipArchiveReader = ZipArchive<BufReader<File>>;

        /// The public view of a weather archive file.
        #[derive(Debug)]
        pub(in crate::backend) struct WeatherHistory(
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
            pub(in crate::backend) fn new(alias: &str, file: WeatherFile) -> Result<Self> {
                let archive = WeatherArchive::open(alias, file)?;
                Ok(Self(archive))
            }
            /// Creates a summary of the weather history statistics.
            pub(in crate::backend) fn summary(&self) -> Result<HistorySummary> {
                let mut files: usize = 0;
                let mut size: u64 = 0;
                let mut compressed_size: u64 = 0;
                let iter = self.0.archive_iter(None, false, ArchiveMd::new)?;
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
            pub(in crate::backend) fn dates(&self) -> Result<DateRanges> {
                let mut stopwatch = StopWatch::start_new();
                let iter = self.0.archive_iter(None, false, ArchiveMd::new)?;
                let dates: Vec<NaiveDate> = iter.map(|md| md.date).collect();
                log::trace!("WeatherHistory: collect dates {}", &stopwatch);
                stopwatch.start();
                let date_ranges = DateRange::from_dates(dates);
                log::trace!(
                    "WeatherHistory: collect date_ranges {}us",
                    toolslib::fmt::commafy(stopwatch.elapsed().as_micros())
                );
                Ok(DateRanges { location_id: self.0.alias.to_string(), date_ranges })
            }
            /// Get an iterator of daily weather history for a location.
            ///
            /// # Arguments
            ///
            /// * `filter` restricts the range of the historical weather data.
            ///
            pub(in crate::backend) fn daily_histories(&self, filter: &DateRange) -> Result<Vec<History>> {
                // let iter = self.0.archive_iter(Some(filter), true, daily_history_builder)?;
                fn history_builder(alias: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<History> {
                    let data = ArchiveData::new(alias, date, zipfile)?;
                    history::from_bytes(alias, data.bytes())
                }
                let iter = self.0.archive_iter(Some(filter), true, history_builder)?;
                let histories = iter.collect();
                Ok(histories)
            }
        }

        /// The weather archive file updater.
        #[derive(Debug)]
        pub(in crate::backend) struct WeatherHistoryUpdate(
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
            pub(in crate::backend) fn new(alias: &str, file: WeatherFile) -> Result<Self> {
                let archive = WeatherArchive::open(alias, file)?;
                Ok(Self(archive))
            }
            /// Add histories to the weather archive that don't already exist.
            ///
            /// # Arguments
            ///
            /// * `histories` are the histories that will be added.
            pub(in crate::backend) fn add(&mut self, histories: &Vec<History>) -> Result<usize> {
                // don't add histories that already exist
                let mut stopwatch = StopWatch::start_new();
                let dates_iter = self.0.archive_iter(None, false, ArchiveMd::new)?;
                let existing_dates: HashSet<NaiveDate> = dates_iter.map(|md| md.date).collect();
                let additions: Vec<&History> = histories
                    .iter()
                    .filter_map(|h| match existing_dates.contains(&h.date) {
                        true => {
                            log::warn!("Location {} already has history for {}.", self.0.alias, h.date);
                            None
                        }
                        false => Some(h),
                    })
                    .collect();
                log::debug!("collect additions {}", &stopwatch);
                // now add the histories that weren't found to the archive
                stopwatch.start();
                let additions_len = additions.len();
                if additions_len > 0 {
                    let mut writer = self.0.archive_writer();
                    writer.write(additions)?;
                }
                log::debug!("archive update added {} in {}", additions_len, &stopwatch);
                Ok(additions_len)
            }
        }

        /// The manager for a `Zip` archive with weather data.
        #[derive(Debug)]
        pub(in crate::backend) struct WeatherArchive {
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
            pub(in crate::backend) fn open(alias: &str, mut file: WeatherFile) -> Result<Self> {
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
            pub(in crate::backend) fn create(alias: &str, mut file: WeatherFile) -> Result<Self> {
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
            pub(in crate::backend) fn archive_iter<T>(
                &self,
                filter: Option<&DateRange>,
                sort: bool,
                builder: HistoryBuilder<T>,
            ) -> Result<ArchiveIter<T>> {
                let inner = self.file.reader()?;
                match ZipArchive::new(BufReader::new(inner)) {
                    Ok(mut reader) => {
                        let mut history_dates = Self::filter_history(&mut reader, filter);
                        if sort {
                            history_dates.sort()
                        }
                        Ok(ArchiveIter::new(&self.alias, reader, history_dates, builder))
                    }
                    Err(err) => Err(archive_err!(&self.alias, &format!("get_reader error ({}).", &err))),
                }
            }
            pub(in crate::backend) fn archive_writer(&mut self) -> ArchiveWriter {
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
            fn filter_history(reader: &mut ZipArchiveReader, filter: Option<&DateRange>) -> Vec<NaiveDate> {
                let stopwatch = StopWatch::start_new();
                let dates = reader
                    .file_names()
                    .filter_map(|filename| match WeatherArchive::filename_to_date(filename) {
                        Ok(date) => Some(date),
                        Err(err) => {
                            log::error!("{}", &err);
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
        pub(in crate::backend) struct ArchiveMd {
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
            pub(in crate::backend) fn new(alias: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<Self> {
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
            pub(in crate::backend) fn datetime_to_millis(alias: &str, datetime: DateTime) -> i64 {
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

        // #[deprecated]
        /// A bean providing the contents of a weather history file in the archive.
        #[derive(Debug)]
        pub(in crate::backend) struct ArchiveData {
            /// The location identifier.
            pub alias: String,
            /// The date associated with the history file in the archive.
            pub date: NaiveDate,
            /// The data content buffer.
            data: Vec<u8>,
        }
        impl ArchiveData {
            /// Create a new instance of the weather history file contents.
            ///
            /// # Arguments
            ///
            /// * `alias` is the location identifier.
            /// * `date` is the date of the weather history.
            /// * `zipfile` provides the contents of the history file in the archive.
            pub(in crate::backend) fn new(alias: &str, date: &NaiveDate, mut zipfile: ZipFile) -> Result<Self> {
                let size = zipfile.size() as usize;
                let mut data: Vec<u8> = Vec::with_capacity(size);
                if let Err(err) = zipfile.read_to_end(&mut data) {
                    let reason = format!("error reading {} history ({})", date, err);
                    Err(archive_err!(alias, reason))
                } else {
                    Ok(Self { alias: alias.to_string(), date: date.clone(), data })
                }
            }
            /// Get the file contents as a slice of bytes.
            pub(in crate::backend) fn bytes(&self) -> &[u8] {
                &self.data
            }
            /// Get the file contents as a parsed `JSON` document.
            pub(in crate::backend) fn json(&self) -> Result<json::Value> {
                match serde_json::from_reader(self.bytes()) {
                    Ok(json) => Ok(json),
                    Err(err) => {
                        let reason = format!("{} to JSON error ({})", isodate(&self.date), &err);
                        Err(archive_err!(&self.alias, reason))
                    }
                }
            }
        }

        /// The function signature used by the weather archive iterator to create history data.
        type HistoryBuilder<T> = fn(&str, &NaiveDate, ZipFile) -> Result<T>;

        /// The low-level iterator over weather history in the archive.
        pub(in crate::backend) struct ArchiveIter<T> {
            /// The location identifier.
            alias: String,
            /// The `ZIP` archive reader.
            reader: ZipArchiveReader,
            /// The weather history dates that will be returned.
            dates: Vec<NaiveDate>,
            /// The index of the next weather history date.
            index: usize,
            /// The function used to create the appropriate type of history data.
            make: HistoryBuilder<T>,
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
                Self { alias: alias.to_string(), reader, dates, index: 0, make }
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
                        Ok(zip_file) => match (self.make)(&self.alias, &date, zip_file) {
                            Ok(data) => {
                                history.replace(data);
                            }
                            Err(err) => {
                                let reason = format!("HistoryBuilder error ({})", &err);
                                log::error!("{}", &archive_err!(&self.alias, reason));
                            }
                        },
                        Err(err) => {
                            let reason = format!("by_name error ({})", &err);
                            log::error!("{}", &archive_err!(&self.alias, reason));
                        }
                    }
                }
                history
            }
        }

        /// The manager that adds weather history to an archive.
        #[derive(Debug)]
        pub(in crate::backend) struct ArchiveWriter<'a> {
            /// The archive that will be updated.
            archive: &'a WeatherArchive,
            /// The pathname of the archive that will actually have data added to it.
            writable: PathBuf,
        }
        impl<'a> ArchiveWriter<'a> {
            /// The extension that identifies a writable archive.
            const UPDATE_EXT: &str = "upd";
            /// The extension that identifies an archive backup.
            const BACKUP_EXT: &str = "bu";
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
            /// `histories` is what will be added to the archvie.
            pub(in crate::backend) fn write(&mut self, histories: Vec<&History>) -> Result<()> {
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
                let options = FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .last_modified_time(mtime);
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
                match self.copy(self.archive.file.path(), &backup) {
                    Ok(_) => match fs::rename(&self.writable, &self.archive.file.path()) {
                        Ok(_) => {
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
                                    log::error!("{}: error restoring original archive ({}).", self.archive.alias, err)
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
            /// If something bad happens adding history, this attempts to clean up files that might be
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
                // don't copy files use the test resources... just don't update files!!!
                let resources = testlib::test_resources().join("filesys");
                let weather_dir = WeatherDir::new(resources).unwrap();
                let alias = "testmd";
                // verify the test data
                let file = weather_dir.archive(alias);
                assert!(file.exists());
                // setup the testcase
                let from = get_date(2014, 4, 1);
                let to = get_date(2014, 4, 5);
                let history_range = DateRange::new(from.clone(), to.clone());
                let archive = WeatherArchive::open(alias, file).unwrap();
                // let mut testcase = archive.metadata(Some(&history_range), true).unwrap();
                let mut testcase = archive.archive_iter(Some(&history_range), true, ArchiveData::new).unwrap();
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
                // setup the testcase
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
                let mut iter = archive.archive_iter(None, false, ArchiveMd::new).unwrap();
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

    // use locations::{WeatherLocations, LOCATIONS_FILENAME};
    pub(crate) use locations::create as weather_locations;
    mod locations {
        //! The data model for weather data locations.
        //!
        use super::*;
        use serde_json as json;

        /// The name of the locations document in the weather data directory.
        pub const LOCATIONS_FILENAME: &str = "locations.json";

        /// The [Locations] error builder.
        macro_rules! locations_err {
            ($reason:expr) => {
                Error::from(format!("WeatherLocations: {}", $reason))
            };
        }

        pub(crate) fn create(weather_dir: &WeatherDir) -> Result<Locations> {
            let file = weather_dir.file(LOCATIONS_FILENAME);
            if file.exists() {
                Locations::new(file)
            } else {
                log::warn!("{} does not exist", file);
                Ok(Locations(vec![]))
            }
        }

        /// The locations `JSON` document manager.
        #[derive(Debug)]
        pub(crate) struct Locations(
            /// The locations metadata.
            Vec<LocationMd>,
        );
        impl Locations {
            /// Create a new instance of the manager.
            ///
            /// An error will be returned if the `JSON` document file does not exist.
            ///
            /// # Arguments
            ///
            /// * `file` contains the weather data locations.
            pub fn new(file: WeatherFile) -> Result<Self> {
                let reader = file.reader()?;
                let result: std::result::Result<LocationsMd, json::Error> = json::from_reader(reader);
                match result {
                    Ok(md) => {
                        // let locations: Vec<Location> = md.locations.into_iter().map(|md| Location::from(md)).collect();
                        Ok(Self(md.locations))
                    }
                    Err(err) => {
                        let reason = format!("Error loading JSON from {}: {}", &file, &err);
                        Err(locations_err!(reason))
                    }
                }
            }
            /// Creates an iterator returning weather data locations.
            ///
            /// # Arguments
            ///
            /// * `filters` are used to scope which locations will be returned.
            /// * `icase` will make filters case sensitive (`true`) or ignore case (`false`).
            /// * `sort` will order the matching locations by their name.
            pub fn as_iter(&self, patterns: &Vec<String>, icase: bool, sort: bool) -> LocationsIter {
                let prepare = |text: &str| if icase { text.to_string() } else { text.to_lowercase() };
                let mut locations: Vec<&LocationMd> = if patterns.is_empty() {
                    self.0.iter().collect()
                } else {
                    let patterns: Vec<String> = patterns.iter().map(|pattern| prepare(pattern)).collect();
                    self.0
                        .iter()
                        .filter(|location| {
                            let name = prepare(&location.name);
                            let alias = prepare(&location.alias);
                            patterns.iter().any(|pattern| is_match(&name, &alias, pattern))
                        })
                        .collect()
                };
                if sort {
                    locations.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
                }
                LocationsIter::new(locations)
            }
        }

        /// Test if location name or alias matches a pattern.
        ///
        /// The pattern can include an `*` to denote a match of any leading characters, any trailing characters, or
        /// if the pattern should be considered a sub-string match.
        ///  
        /// # Arguments
        ///
        /// * `name` is the location name that will be matched against the pattern.
        /// * `alias` is the location alias that will be matched against the pattern.
        /// * `pattern` is what will be matched against the location name and alias.
        fn is_match(name: &String, alias: &String, pattern: &String) -> bool {
            if pattern == "*" {
                true
            } else if pattern.starts_with("*") && pattern.ends_with("*") {
                let slice = &pattern[1..pattern.len() - 1];
                name.contains(slice) || alias.contains(slice)
            } else if pattern.starts_with("*") {
                let slice = &pattern[1..];
                name.ends_with(slice) || alias.ends_with(slice)
            } else if pattern.ends_with("*") {
                let slice = &pattern[..pattern.len() - 1];
                name.starts_with(slice) || alias.starts_with(slice)
            } else {
                name == pattern || alias == pattern
            }
        }

        use serde::{Deserialize, Serialize};

        /// The bean that describes the locations `JSON` document.
        #[derive(Debug, Deserialize, Serialize)]
        struct LocationsMd {
            /// The collection of location metadata.
            locations: Vec<LocationMd>,
        }

        /// The bean that describes the metadata for a location.
        #[derive(Debug, Deserialize, Serialize)]
        struct LocationMd {
            /// The name of a location.
            name: String,
            /// A unique nickname of a location.
            alias: String,
            /// The location longitude.
            longitude: String,
            /// The location latitude.
            latitude: String,
            /// the location timezone.
            tz: String,
        }
        impl From<&LocationMd> for Location {
            /// Convert the `JSON` location metadata to a [Location].
            fn from(md: &LocationMd) -> Self {
                Location {
                    name: md.name.clone(),
                    alias: md.alias.clone(),
                    longitude: md.longitude.clone(),
                    latitude: md.latitude.clone(),
                    tz: md.tz.clone(),
                }
            }
        }

        /// An iterator over the `JSON` location metadata.
        #[derive(Debug)]
        pub struct LocationsIter<'l> {
            /// The colelction of `JSON` location metadata.
            location_md: Vec<&'l LocationMd>,
            /// The next location metadata that will be returned.
            index: usize,
        }
        impl<'l> LocationsIter<'l> {
            fn new(location_md: Vec<&'l LocationMd>) -> Self {
                Self { location_md, index: 0 }
            }
        }
        impl<'l> Iterator for LocationsIter<'l> {
            type Item = Location;
            /// Get the next `JSON` location metadata as a [Location].
            fn next(&mut self) -> Option<Self::Item> {
                match self.index < self.location_md.len() {
                    true => {
                        let location_md = self.location_md[self.index];
                        self.index += 1;
                        Some(Location::from(location_md))
                    }
                    false => None,
                }
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn matches() {
                let target = "start".to_string();
                let testcase = "st*".to_string();
                assert!(is_match(&target, &String::default(), &testcase));
                assert!(is_match(&String::default(), &target, &testcase));
                assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
                let target = "end".to_string();
                let testcase = "*d".to_string();
                assert!(is_match(&target, &String::default(), &testcase));
                assert!(is_match(&String::default(), &target, &testcase));
                assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
                let target = "middle".to_string();
                let testcase = "*dd*".to_string();
                assert!(is_match(&target, &String::default(), &testcase));
                assert!(is_match(&String::default(), &target, &testcase));
                assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
                let target = "exact".to_string();
                let testcase = "exact".to_string();
                assert!(is_match(&target, &String::default(), &testcase));
                assert!(is_match(&String::default(), &target, &testcase));
                assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
            }

            #[test]
            fn as_iter() {
                let resources = testlib::test_resources().join("filesys");
                let weather_dir = WeatherDir::new(resources).unwrap();
                let testcase = Locations::new(weather_dir.file("locations.json")).unwrap();
                assert_eq!(testcase.0.len(), 3);
                // no filters
                let mut result = testcase.as_iter(&vec![], false, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Northern City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
                // starts with
                let patterns = vec!["Bet*".to_string(), "nOrth*".to_string(), "South*".to_string()];
                let mut result = testcase.as_iter(&patterns, true, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
                let mut result = testcase.as_iter(&patterns, false, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Northern City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
                // ends with
                let patterns = vec!["*en City".to_string(), "*RN city".to_string()];
                let mut result = testcase.as_iter(&patterns, true, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert!(result.next().is_none());
                let mut result = testcase.as_iter(&patterns, false, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Northern City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
                // contains
                let patterns = vec!["*et*".to_string(), "*OUT*".to_string()];
                let mut result = testcase.as_iter(&patterns, true, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert!(result.next().is_none());
                let mut result = testcase.as_iter(&patterns, false, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
                // exact
                let patterns = vec!["north".to_string(), "South".to_string(), "between".to_string()];
                let mut result = testcase.as_iter(&patterns, true, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Northern City");
                assert!(result.next().is_none());
                let mut result = testcase.as_iter(&patterns, false, true);
                assert_eq!(&result.next().unwrap().name, "Between City");
                assert_eq!(&result.next().unwrap().name, "Northern City");
                assert_eq!(&result.next().unwrap().name, "Southern City");
                assert!(result.next().is_none());
            }

            #[test]
            fn from() {
                let md = LocationMd {
                    name: "Name".to_string(),
                    alias: "Alias".to_string(),
                    longitude: "1.23".to_string(),
                    latitude: "-10.3".to_string(),
                    tz: "UTC".to_string(),
                };
                let testcase = Location::from(&md);
                assert_eq!(testcase.name, md.name);
                assert_eq!(testcase.alias, md.alias);
                assert_eq!(testcase.longitude, md.longitude);
                assert_eq!(testcase.latitude, md.latitude);
                assert_eq!(testcase.tz, md.tz);
            }
        }
    }

    pub(crate) use adapter::create as archive_adapter;
    mod adapter {
        //! The archive based implementation of the [DataAdapter].

        use super::{weather_dir, weather_locations, WeatherDir, WeatherHistory, WeatherHistoryUpdate};
        use crate::{
            backend::{DataAdapter, Result},
            prelude::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location},
        };
        use toolslib::stopwatch::StopWatch;

        /// Creates the file based data API for weather data.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the weather data directory name. If the directory name is
        /// empty the `WEATHER_DATA` environment varibale will be used if it has been
        /// defined. Otherwise it will use the `weather_data` directory in the current
        /// working directory.
        pub(crate) fn create(dirname: &str) -> Result<Box<dyn DataAdapter>> {
            Ok(Box::new(ArchiveDataAdapter(weather_dir(dirname)?)))
        }

        /// Consolidate logging elapsed time here.
        macro_rules! log_elapsed {
            (trace, $what:expr, $stopwatch:expr) => {
                log::trace!("ArchiveDataAdapter: {} {}", $what, $stopwatch)
            };
            ($what:expr, $stopwatch:expr) => {
                log::debug!("ArchiveDataAdapter: {} {}", $what, $stopwatch)
            };
        }

        /// The archive implemenation of a [DataAdapter].
        pub(in crate::backend) struct ArchiveDataAdapter(
            /// The directory containing weather data files
            WeatherDir,
        );
        impl ArchiveDataAdapter {
            /// Used internally to get the archive manager for some location.
            ///
            /// # Arguments
            ///
            /// * `alias` is the location identifier.
            fn get_archive(&self, alias: &str) -> Result<WeatherHistory> {
                let mut stopwatch = StopWatch::start_new();
                let weather_file = self.0.archive(alias);
                log_elapsed!(trace, format!("get_archive '{}' WeatherFile", alias), &stopwatch);
                stopwatch.start();
                let weather_history = WeatherHistory::new(alias, weather_file)?;
                log_elapsed!(trace, format!("get_archive '{}' WeatherHistory", alias), &stopwatch);
                Ok(weather_history)
            }
        }
        impl DataAdapter for ArchiveDataAdapter {
            /// Returns the daily weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `location` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, location: Location, history_range: DateRange) -> Result<DailyHistories> {
                let stopwatch = StopWatch::start_new();
                let archive = self.get_archive(&location.alias)?;
                let daily_histories = archive.daily_histories(&history_range)?;
                log_elapsed!("daily_histories", &stopwatch);
                Ok(DailyHistories { location, histories: daily_histories })
            }
            /// Get the weather history dates for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations.
            fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
                let locations = self.locations(criteria)?;
                let stopwatch = StopWatch::start_new();
                let mut history_dates = Vec::with_capacity(locations.len());
                for location in locations {
                    let inner_stopwatch = StopWatch::start_new();
                    let archive = self.get_archive(&location.alias)?;
                    let dates = archive.dates()?;
                    log_elapsed!(trace, &format!("{} history dates", location.alias), &inner_stopwatch);
                    history_dates.push(HistoryDates { location, history_dates: dates.date_ranges })
                }
                log_elapsed!("history_dates", &stopwatch);
                Ok(history_dates)
            }
            /// Get the summary metrics of a locations weather data.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations that should be used.
            fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
                let locations = self.locations(criteria)?;
                let stopwatch = StopWatch::start_new();
                let mut history_summaries = Vec::with_capacity(locations.len());
                for location in locations {
                    let archive = self.get_archive(&location.alias)?;
                    let summary = archive.summary()?;
                    history_summaries.push(HistorySummaries {
                        location,
                        count: summary.count,
                        overall_size: summary.overall_size,
                        raw_size: summary.raw_size,
                        store_size: summary.compressed_size,
                    });
                }
                log_elapsed!("history_summaries", &stopwatch);
                Ok(history_summaries)
            }
            /// Get the metadata for weather locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations of interest.
            fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
                let stopwatch = StopWatch::start_new();
                let weather_locations = weather_locations(&self.0)?;
                let locations = weather_locations.as_iter(&criteria.filters, !criteria.icase, criteria.sort).collect();
                log_elapsed!("locations", &stopwatch);
                Ok(locations)
            }
            /// Add weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `daily_histories` has the loation and histories to add.
            fn add_histories(&self, daily_histories: &DailyHistories) -> Result<usize> {
                let location = &daily_histories.location;
                let file = self.0.archive(&location.alias);
                let mut archive_updater = WeatherHistoryUpdate::new(&location.alias, file)?;
                let additions = archive_updater.add(&daily_histories.histories)?;
                Ok(additions)
            }
        }
    }
}

pub(crate) use admin::{migrate_history, MigrateConfig};
mod admin {
    //! Isolates the adminstration API from the weather API.

    use super::*;
    use crate::{
        backend::{Error, Result},
        entities::{DataCriteria, History, Location},
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use std::{
        env, fs,
        io::Read,
        path::{Path, PathBuf},
    };
    use zip::read::ZipFile;

    /// The common error
    macro_rules! error {
        ($reason:expr) => {
            Err(Error::from($reason))
        };
    }

    #[derive(Debug)]
    /// The metadata surrounding migrating old data to [History].
    pub(crate) struct MigrateConfig<'w> {
        /// The old history weather data directory.
        pub source: &'w WeatherDir,
        /// Create the target weather data directory if it does not exist.
        pub create: bool,
        /// Do not delete existing data in the target archive.
        pub retain: bool,
        /// The locations that will be migrated.
        pub criteria: DataCriteria,
    }

    /// Migrate existing weather history to [History].
    ///
    /// # Arguments
    ///
    /// * `config` is the migration configuration.
    /// * `target` is the weather data directory where archives will be updated.
    pub(crate) fn migrate_history(config: MigrateConfig, target: PathBuf) -> Result<usize> {
        let to_path = verify_target(&target, config.create)?;
        let from_path = canonicalize_path(config.source.path())?;
        if to_path == from_path {
            error!("The target directory cannot be the same as the weather data directory.")
        } else {
            let target = if env::consts::OS == "windows" {
                // remove the \\?\ from the windows path, it's annoying
                WeatherDir::new(PathBuf::from(&to_path.display().to_string()[4..]))?
            } else {
                WeatherDir::new(to_path)?
            };
            let locations = weather_locations(config.source)?
                .as_iter(&config.criteria.filters, config.criteria.icase, config.criteria.sort)
                .collect::<Vec<Location>>();
            darksky::migrate(config.source, &target, &locations, config.retain)
        }
    }

    /// Make sure the target directory can be used.
    ///
    /// # Arguments
    ///
    /// * `target` is the target weather data directory.
    /// * `create` indicates the target directory should be created if it does not exist.
    fn verify_target(target: &PathBuf, create: bool) -> Result<PathBuf> {
        match target.exists() {
            true => canonicalize_path(target),
            false => match create {
                true => match fs::create_dir_all(target) {
                    Ok(_) => canonicalize_path(target),
                    Err(err) => {
                        let reason = format!("Error creating {} ({}).", target.display(), err);
                        error!(reason)
                    }
                },
                false => {
                    let reason = format!("The directory '{}' does not exist.", target.display());
                    error!(reason)
                }
            },
        }
    }

    /// Creates an absolute path to the weather data directory.
    ///
    /// # Arguments
    ///
    /// * `path` is the directory name.
    fn canonicalize_path(path: &Path) -> Result<PathBuf> {
        match fs::canonicalize(path) {
            Ok(path) => Ok(path),
            Err(err) => {
                let reason = format!("Error getting absolute path for {} ({}).", path.display(), err);
                error!(reason)
            }
        }
    }

    mod darksky {
        //! Isolate the Darksky conversion to this module.
        use super::*;
        use serde::{Deserialize, Serialize};

        /// The entry point to migrate *DarkSky* weather history to [History].
        ///
        /// # Arguments
        ///
        /// * `source_dir` is the *DarkSky* weather data directory.
        /// * `target_dir` is where migrated weather data will be written.
        /// * `locations` identifies what data will be migrated.
        /// * `retain` indicates existing data in the target archive should not be deleted.
        pub(super) fn migrate(
            source_dir: &WeatherDir,
            target_dir: &WeatherDir,
            locations: &Vec<Location>,
            retain: bool,
        ) -> Result<usize> {
            let mut migrate_count = 0;
            for location in locations {
                let source = WeatherArchive::open(&location.alias, source_dir.archive(&location.alias))?;
                let target = target_archive(&location.alias, target_dir.archive(&location.alias), retain)?;
                let count = migrate_archive(&location.alias, source, target)?;
                migrate_count += count;
            }
            Ok(migrate_count)
        }

        /// Prepares the target weather data archive.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location alias name.
        /// * `weather_file` is the target archive that will be updated.
        /// * `retain` indicates existing data in the target archive should not be deleted.
        fn target_archive(alias: &str, weather_file: WeatherFile, retain: bool) -> Result<WeatherArchive> {
            if !weather_file.exists() {
                WeatherArchive::create(alias, weather_file)
            } else if retain {
                WeatherArchive::open(alias, weather_file)
            } else {
                // remove_file can be lazy so rename the file before deleting
                let old = weather_file.path().with_extension("old");
                if let Err(err) = fs::rename(weather_file.path(), &old) {
                    let reason = format!("Could not rename '{}' to '{}' ({}).", &weather_file, old.display(), err);
                    error!(reason)
                } else if let Err(err) = fs::remove_file(&old) {
                    let reason = format!("Could not remove '{}' ({}).", old.display(), err);
                    error!(reason)
                } else {
                    WeatherArchive::create(alias, weather_file)
                }
            }
        }

        /// Migrate existing weather history to [History].
        ///
        /// # Arguments
        ///
        /// * `alias` is the locations alias name.
        /// * `source` is the source weather history archive.
        /// * `target` is the target weather history archive.
        fn migrate_archive(alias: &str, source: WeatherArchive, mut target: WeatherArchive) -> Result<usize> {
            // need to think more about exposing the alias name in the weather archive.
            log::info!("Migrating '{}'", alias);
            let migrations: Vec<MigrationData> = source.archive_iter(None, false, MigrationData::new)?.collect();
            let mut histories: Vec<History> = Vec::with_capacity(migrations.len());
            for md in migrations {
                let darksky = md.to_darksky()?;
                histories.push(darksky.into_history(alias, &md.date));
            }
            let mut archive_writer = target.archive_writer();
            archive_writer.write(histories.iter().map(|h| h).collect())?;
            Ok(histories.len())
        }

        #[derive(Debug)]
        /// The metadata used to migrate existing weather history.
        struct MigrationData {
            /// The locations alias name.
            alias: String,
            /// The date associated with the weather history.
            date: NaiveDate,
            /// The weather history data.
            data: Vec<u8>,
        }
        impl MigrationData {
            /// Used by the archive iterator to create an instance of the migration metadata.
            ///
            /// # Arguments
            ///
            /// * `alias` is the locations alias name.
            /// * `date` is the weather history date.
            /// * `zipfile` is the archive file holding weather history.
            pub fn new(alias: &str, date: &NaiveDate, mut zipfile: ZipFile) -> Result<Self> {
                let size = zipfile.size() as usize;
                let mut data: Vec<u8> = Vec::with_capacity(size);
                if let Err(err) = zipfile.read_to_end(&mut data) {
                    let reason = format!("MigrationData ({}): error reading {} history ({})", alias, date, err);
                    error!(reason)
                } else {
                    Ok(Self { alias: alias.to_string(), date: date.clone(), data })
                }
            }
            /// Deserialize the history data into [DarkskyHistory].
            fn to_darksky(&self) -> Result<DarkskyHistory> {
                match serde_json::from_slice::<DarkskyHistory>(&self.data) {
                    Ok(history) => Ok(history),
                    Err(err) => {
                        let reason = format!(
                            "MigrationData ({}): {} error creating Darksky history ({})",
                            self.alias, self.date, err
                        );
                        error!(reason)
                    }
                }
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        /// The *DarkSky* document.
        struct DarkskyHistory {
            daily: DarkskyDaily,
            hourly: DarkskyHourly,
            latitude: f64,
            longitude: f64,
            offset: i64,
            timezone: String,
        }
        /// Returns a reference to the daily weather history.
        macro_rules! daily {
            ($self:expr) => {
                &$self.daily.data[0]
            };
        }
        /// Returns an iterator to the hourly weather history.
        macro_rules! hourly_iter {
            ($self:expr) => {
                $self.hourly.data.iter()
            };
        }
        /// Consolidate what happens if weather history cannot be derived.
        macro_rules! map_or_default {
            ($option:expr, $what:literal) => {
                match $option {
                    Some(value) => Some(value),
                    None => {
                        log::trace!("{} has no value, using default.", $what);
                        None
                    }
                }
            };
        }
        impl DarkskyHistory {
            /// Convert *DarkSky* weather history into [History].
            ///
            /// # Arguments
            ///
            /// * `alias` is the location alias name.
            /// * `date` is the weather history date.
            fn into_history(&self, alias: &str, date: &NaiveDate) -> History {
                let daily = daily!(self);
                History {
                    alias: alias.to_string(),
                    date: date.clone(),
                    temperature_high: self.temperature_high(),
                    temperature_low: self.temperature_low(),
                    temperature_mean: self.temperature_mean(),
                    dew_point: self.dew_point(),
                    humidity: self.humidity(),
                    precipitation_chance: self.precipitation_chance(),
                    precipitation_type: daily.precipType.clone(),
                    precipitation_amount: self.precip(),
                    wind_speed: self.wind_speed(),
                    wind_gust: self.wind_gust(),
                    wind_direction: self.wind_bearing(),
                    cloud_cover: self.cloud_cover(),
                    pressure: self.pressure(),
                    uv_index: self.uv_index(),
                    sunrise: daily.sunriseTime.map_or(None, |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                    sunset: daily.sunsetTime.map_or(None, |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                    moon_phase: daily.moonPhase,
                    visibility: self.visibility(),
                    description: daily.summary.clone(),
                }
            }
            /// Extracts the daily high temperature
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            ///
            /// * daily `temperatureHigh`
            /// * daily `temperatureMax`
            /// * daily `apparentTemperatureHigh`
            /// * daily `apparentTemperatureMax`
            /// * hourly `temperature`
            fn temperature_high(&self) -> Option<f64> {
                let daily = daily!(self);
                match daily.temperatureHigh {
                    Some(t) => Some(t),
                    None => match daily.temperatureMax {
                        Some(t) => Some(t),
                        None => match daily.apparentTemperatureHigh {
                            Some(t) => Some(t),
                            None => match daily.apparentTemperatureMax {
                                Some(t) => Some(t),
                                None => {
                                    let temp = hourly_iter!(self).filter_map(|h| h.temperature).reduce(f64::max);
                                    map_or_default!(temp, "temperature_max")
                                }
                            },
                        },
                    },
                }
            }
            /// Extracts the daily low temperature
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            ///
            /// * daily `temperatureLow`
            /// * daily `temperatureMin`
            /// * daily `apparentTemperatureMin`
            /// * daily `apparentTemperatureMin`
            /// * hourly `temperature`
            fn temperature_low(&self) -> Option<f64> {
                let daily = daily!(self);
                match daily.temperatureLow {
                    Some(t) => Some(t),
                    None => match daily.temperatureMin {
                        Some(t) => Some(t),
                        None => match daily.apparentTemperatureMin {
                            Some(t) => Some(t),
                            None => match daily.apparentTemperatureMin {
                                Some(t) => Some(t),
                                None => {
                                    let temp = hourly_iter!(self).filter_map(|h| h.temperature).reduce(f64::min);
                                    map_or_default!(temp, "temperature_min")
                                }
                            },
                        },
                    },
                }
            }
            /// Calculate the daily mean temperature from hourly history.
            fn temperature_mean(&self) -> Option<f64> {
                let temps: Vec<f64> = hourly_iter!(self).filter_map(|h| h.temperature).collect();
                if temps.is_empty() {
                    log::trace!("temperature_mean has no value, using default");
                    None
                } else {
                    let mean_temp = temps.iter().sum::<f64>() / temps.len() as f64;
                    Some((mean_temp * 100.0).round() / 100.0)
                }
            }
            /// Extract the daily dew point.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `dewPoint`
            /// * hourly `dewPoint`
            fn dew_point(&self) -> Option<f64> {
                match daily!(self).dewPoint {
                    Some(dew_point) => Some(dew_point),
                    None => {
                        let dew_point = hourly_iter!(self).filter_map(|h| h.dewPoint).reduce(f64::max);
                        map_or_default!(dew_point, "dew_point")
                    }
                }
            }
            /// Extract the daily humidity.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `humidity`
            /// * hourly `humidity`
            fn humidity(&self) -> Option<f64> {
                match daily!(self).humidity {
                    Some(humidity) => Some(humidity),
                    None => {
                        let humidity = hourly_iter!(self).filter_map(|h| h.humidity).reduce(f64::max);
                        map_or_default!(humidity, "humidity")
                    }
                }
            }
            /// Extract the chance of precipitation.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `precipProbability`
            /// * hourly `precipProbability`
            fn precipitation_chance(&self) -> Option<f64> {
                match daily!(self).precipProbability {
                    Some(chance) => Some(chance),
                    None => {
                        let probablities: Vec<f64> = hourly_iter!(self).filter_map(|h| h.precipProbability).collect();
                        if probablities.is_empty() {
                            log::trace!("probabilities has no value, using default.");
                            None
                        } else {
                            let chance = probablities.iter().sum::<f64>() / probablities.len() as f64;
                            Some((chance * 100.0).round() / 100.0)
                        }
                    }
                }
            }
            /// Extract the amount of precipitation.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `precipIntensity`
            /// * hourly `precipIntensity`
            fn precip(&self) -> Option<f64> {
                let precip = match daily!(self).precipIntensity {
                    Some(p) => p * 24 as f64,
                    None => hourly_iter!(self).filter_map(|h| h.precipIntensity).sum::<f64>(),
                };
                Some(precip)
            }
            /// Extract the daily wind speed.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `windSpeed`
            /// * hourly `windSpeed`
            fn wind_speed(&self) -> Option<f64> {
                match daily!(self).windSpeed {
                    Some(speed) => Some(speed),
                    None => {
                        let speeds = hourly_iter!(self).filter_map(|md| md.windSpeed).collect::<Vec<f64>>();
                        if speeds.is_empty() {
                            log::trace!("wind_speed has no value, using default.");
                            None
                        } else {
                            let speed = speeds.iter().sum::<f64>() / speeds.len() as f64;
                            Some((speed * 100.0).round() / 100.0)
                        }
                    }
                }
            }
            /// Extract the daily wind gust speed.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `windGust`
            /// * hourly `windGust`
            fn wind_gust(&self) -> Option<f64> {
                match daily!(self).windGust {
                    Some(gust) => Some(gust),
                    None => {
                        let gust = hourly_iter!(self).filter_map(|md| md.windGust).reduce(f64::max);
                        map_or_default!(gust, "wind_gust")
                    }
                }
            }
            /// Extract the daily wind bearing.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `windBearing`
            /// * hourly `windBearing`
            fn wind_bearing(&self) -> Option<i64> {
                match daily!(self).windBearing {
                    Some(bearing) => Some(bearing),
                    None => {
                        let bearings = hourly_iter!(self).filter_map(|md| md.windBearing).collect::<Vec<i64>>();
                        if bearings.is_empty() {
                            log::trace!("wind_bearing has no value, using default.");
                            None
                        } else {
                            let bearing = bearings.iter().sum::<i64>() as f64 / bearings.len() as f64;
                            Some(bearing.round() as i64)
                        }
                    }
                }
            }
            /// Extract the daily cloud cover.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `cloudCover`
            /// * hourly `cloudCover`
            fn cloud_cover(&self) -> Option<f64> {
                match daily!(self).cloudCover {
                    Some(cover) => Some(cover),
                    None => {
                        let covers = hourly_iter!(self).filter_map(|md| md.cloudCover).collect::<Vec<f64>>();
                        if covers.is_empty() {
                            log::trace!("cloud_cover has no value, using default.");
                            None
                        } else {
                            let cover = covers.iter().sum::<f64>() / covers.len() as f64;
                            Some((cover * 100.0).round() / 100.0)
                        }
                    }
                }
            }
            /// Extract the daily atmospheric pressure.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `pressure`
            /// * hourly `pressure`
            fn pressure(&self) -> Option<f64> {
                match daily!(self).pressure {
                    Some(pressure) => Some(pressure),
                    None => {
                        let pressures = hourly_iter!(self).filter_map(|md| md.pressure).collect::<Vec<f64>>();
                        if pressures.is_empty() {
                            log::trace!("pressure has no value, using default.");
                            None
                        } else {
                            let pressure = pressures.iter().sum::<f64>() / pressures.len() as f64;
                            Some((pressure * 10000.0).round() / 10000.0)
                        }
                    }
                }
            }
            /// Extract the daily atmospheric pressure.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `uvIndex`
            /// * hourly `uvIndex`
            fn uv_index(&self) -> Option<f64> {
                match daily!(self).uvIndex {
                    Some(uv_index) => Some(uv_index as f64),
                    None => {
                        let uv_indexes = hourly_iter!(self).filter_map(|md| md.uvIndex).collect::<Vec<i64>>();
                        if uv_indexes.is_empty() {
                            log::trace!("uv_index has no value, using default.");
                            None
                        } else {
                            let uv_index = uv_indexes.iter().sum::<i64>() as f64 / uv_indexes.len() as f64;
                            Some((uv_index * 100.0).round() / 100.0)
                        }
                    }
                }
            }
            /// Extract the daily atmospheric pressure.
            ///
            /// *DarkSky* history is sparse so the following attributes are examined from first to last.
            /// * daily `visibility`
            /// * hourly `visibility`
            fn visibility(&self) -> Option<f64> {
                match daily!(self).visibility {
                    Some(visibility) => Some(visibility),
                    None => {
                        let visibilities = hourly_iter!(self).filter_map(|md| md.visibility).collect::<Vec<f64>>();
                        if visibilities.is_empty() {
                            log::trace!("visibility has no value, using default.");
                            None
                        } else {
                            let visibility = visibilities.iter().sum::<f64>() / visibilities.len() as f64;
                            Some((visibility * 100.0).round() / 100.0)
                        }
                    }
                }
            }
        }
        #[derive(Debug, Serialize, Deserialize)]
        /// The *DarkSky* hourly weather history data.
        struct DarkskyHourly {
            data: Vec<HourlyMd>,
        }
        #[allow(non_snake_case)]
        #[derive(Debug, Serialize, Deserialize)]
        /// The *DarkSky* hourly weather metadata.
        struct HourlyMd {
            apparentTemperature: Option<f64>,
            cloudCover: Option<f64>,
            dewPoint: Option<f64>,
            humidity: Option<f64>,
            icon: Option<String>,
            precipIntensity: Option<f64>,
            precipProbability: Option<f64>,
            pressure: Option<f64>,
            summary: Option<String>,
            temperature: Option<f64>,
            time: Option<i64>,
            uvIndex: Option<i64>,
            visibility: Option<f64>,
            windBearing: Option<i64>,
            windGust: Option<f64>,
            windSpeed: Option<f64>,
        }
        #[derive(Debug, Serialize, Deserialize)]
        /// The *DarkSky* dailty weather history data.
        struct DarkskyDaily {
            data: Vec<DailyMd>,
        }
        #[allow(non_snake_case)]
        #[derive(Debug, Serialize, Deserialize)]
        /// The *DarkSky* daily weather metadata.
        struct DailyMd {
            apparentTemperatureHigh: Option<f64>,
            apparentTemperatureHighTime: Option<i64>,
            apparentTemperatureLow: Option<f64>,
            apparentTemperatureLowTime: Option<i64>,
            apparentTemperatureMax: Option<f64>,
            apparentTemperatureMaxTime: Option<i64>,
            apparentTemperatureMin: Option<f64>,
            apparentTemperatureMinTime: Option<i64>,
            cloudCover: Option<f64>,
            dewPoint: Option<f64>,
            humidity: Option<f64>,
            icon: Option<String>,
            moonPhase: Option<f64>,
            precipIntensity: Option<f64>,
            precipIntensityMax: Option<f64>,
            precipIntensityMaxTime: Option<f64>,
            precipProbability: Option<f64>,
            precipType: Option<String>,
            pressure: Option<f64>,
            summary: Option<String>,
            sunriseTime: Option<i64>,
            sunsetTime: Option<i64>,
            temperatureHigh: Option<f64>,
            temperatureHighTime: Option<i64>,
            temperatureLow: Option<f64>,
            temperatureLowTime: Option<i64>,
            temperatureMax: Option<f64>,
            temperatureMaxTime: Option<i64>,
            temperatureMin: Option<f64>,
            temperatureMinTime: Option<i64>,
            time: Option<i64>,
            uvIndex: Option<i64>,
            uvIndexTime: Option<i64>,
            visibility: Option<f64>,
            windBearing: Option<i64>,
            windGust: Option<f64>,
            windGustTime: Option<i64>,
            windSpeed: Option<f64>,
        }
    }
}
