//! The filesystem objects that support implementing weather data using `ZIP` archives.

pub(crate) use v1::{
    archive_adapter, archive_name, to_daily_history, to_json, weather_dir, weather_locations, ArchiveData, ArchiveMd,
    WeatherArchive, WeatherDir, WeatherFile, WeatherHistory,
};

mod v1 {
    //! The first generation of the new file based weather data implmentation
    //!
    use crate::backend::{Error, Result};
    use crate::prelude::{DailyHistory, DateRange, DateRanges, HistorySummary, Location};
    use std::{
        fmt::Display,
        fs::File,
        io::BufReader,
        path::{Path, PathBuf},
    };

    pub(crate) fn weather_dir(dirname: &str) -> Result<WeatherDir> {
        let weather_dir = if dirname.len() > 0 {
            WeatherDir::new(dirname)?
        } else if let Ok(env_pathname) = std::env::var("WEATHER_DATA") {
            WeatherDir::new(&env_pathname)?
        } else {
            WeatherDir::new("weather_data")?
        };
        Ok(weather_dir)
    }

    #[inline]
    pub fn archive_name(lid: &str) -> String {
        format!("{}.zip", lid)
    }

    // expose the file utilities to the module
    pub(crate) use file::{WeatherDir, WeatherFile};
    mod file {
        //! Support for filesystem access.
        //!
        use super::*;
        use std::{
            fs::{read_dir, Metadata, OpenOptions},
            time::{Duration, SystemTime},
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
        impl WeatherDir {
            /// Creates a new instance of the weather directory manager.
            ///
            /// An error will be returned if the directory does not exist or it does exist but is not a directory.
            ///
            /// # Arguments
            ///
            /// * `directory_name` is the name of the directory.
            pub fn new(directory_name: &str) -> Result<WeatherDir> {
                let path = PathBuf::from(directory_name);
                match path.is_dir() {
                    true => Ok(WeatherDir(path)),
                    false => Err(dir_err!(directory_name, "Not a directory...")),
                }
            }
            /// Get a weather file from within the managed directory.
            ///
            /// # Arguments
            ///
            /// * `filename` is the name of the file within the weather directory.
            pub fn get_file(&self, filename: &str) -> WeatherFile {
                WeatherFile::new(self.0.join(filename))
            }
            #[allow(unused)]
            /// Return the contents of the directory as a collection of weather files.
            pub fn get_files(&self) -> Vec<WeatherFile> {
                match read_dir(&self.0) {
                    Ok(reader) => reader
                        .into_iter()
                        .filter_map(|result| match result {
                            Ok(dir_entry) => {
                                let file = WeatherFile::new(dir_entry.path());
                                Some(file)
                            }
                            Err(err) => {
                                log::error!("{}", &dir_err!(self, format!("DirEntry error ({})", &err)));
                                None
                            }
                        })
                        .collect(),
                    Err(err) => {
                        log::error!("{}", &dir_err!(self, format!("read_dir error ({})...", &err)));
                        vec![]
                    }
                }
            }
        }

        /// The [WeatherFile] error builder.
        macro_rules! file_err {
            ($id:expr, $reason:expr) => {
                Error::from(format!("WeatherFile ({}): {}", $id, $reason))
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
                write!(f, "{}", self.path.as_path().display())
            }
        }
        impl WeatherFile {
            /// Create the manager for files in the weather directory.
            ///
            /// # Arguments
            ///
            /// * `path` is the weather data file returned by the [`WeatherDir`].
            fn new(path: PathBuf) -> Self {
                // this has to work in this use case because the path comes from a DirEntry
                let name = path.file_name().unwrap().to_str().unwrap().to_string();
                let fs_metadata = if path.exists() { Self::stat(&path) } else { None };
                WeatherFile { filename: name, path, fs_metadata }
            }
            /// Refresh the filesystem metadata.
            pub fn refresh(&mut self) {
                self.fs_metadata = Self::stat(&self.path);
            }
            /// Indicates if the file exists or does not.
            pub fn exists(&self) -> bool {
                self.fs_metadata.is_some()
            }
            /// Gets the file modification time as an offset from the Unix epoch.
            #[allow(unused)]
            pub fn mtime(&self) -> i64 {
                let zero = Duration::ZERO;
                let epoch = SystemTime::UNIX_EPOCH;
                let fs_mtime = match &self.fs_metadata {
                    Some(md) => md.modified().unwrap_or(epoch).duration_since(epoch).unwrap_or(zero),
                    None => zero,
                };
                fs_mtime.as_millis() as i64
            }
            /// Get the size of the file.
            pub fn size(&self) -> u64 {
                match &self.fs_metadata {
                    Some(md) => md.len(),
                    None => 0,
                }
            }
            /// Get the writer that can be used to update a Zip archive.
            pub fn writer(&self) -> Result<File> {
                match File::options().read(true).write(true).open(&self.path) {
                    Ok(file) => Ok(file),
                    Err(err) => Err(file_err!(&self.filename, &format!("append error ({}).", &err))),
                }
            }
            /// Get the reader that can be used to read the contents of an Zip archive.
            pub fn reader(&self) -> Result<BufReader<File>> {
                match OpenOptions::new().read(true).open(&self.path) {
                    Ok(file) => Ok(BufReader::new(file)),
                    Err(err) => {
                        eprintln!("error: {:?}", &self.path);
                        Err(file_err!(&self.filename, &format!("open read error ({})...", &err)))
                    }
                }
            }
            /// Used internally to get the filesystem metadata for the file.
            fn stat(path: &Path) -> Option<Metadata> {
                match path.metadata() {
                    Ok(md) => Some(md),
                    Err(err) => {
                        log::error!("{}", &file_err!(&path.display(), &err));
                        None
                    }
                }
            }
        }

        #[cfg(test)]
        mod test {
            use super::*;
            use std::io::{Read, Write};

            #[test]
            fn weather_file() {
                let fixture = testutil::TestFixture::create();
                let filename = "testfile.dat";
                let mut testcase = WeatherFile::new(fixture.create_path(filename));
                // verify metadata for a file that does not exist
                assert_eq!(testcase.filename, filename);
                assert!(!testcase.exists());
                assert_eq!(testcase.mtime(), 0);
                assert_eq!(testcase.size(), 0);
                // create the file and content
                let content = "testcase file content...";
                {
                    OpenOptions::new().create(true).write(true).open(&testcase.to_string()).unwrap();
                    testcase.writer().unwrap().write_all(content.as_bytes()).unwrap();
                }
                // verify metadata behaviour
                assert!(!testcase.exists());
                assert_eq!(testcase.mtime(), 0);
                assert_eq!(testcase.size(), 0);
                testcase.refresh();
                assert_eq!(testcase.filename, filename);
                assert!(testcase.exists());
                assert!(testcase.mtime() > 0);
                assert_eq!(testcase.size(), content.len() as u64);
                let mtime = SystemTime::now().checked_sub(Duration::new(10, 0)).unwrap();
                fixture.set_modtime(filename, &mtime);
                testcase.refresh();
                let millis = mtime.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64;
                assert_eq!(testcase.mtime(), millis);
                // verify reading the file content
                {
                    let mut reader = testcase.reader().unwrap();
                    let mut file_content = String::new();
                    reader.read_to_string(&mut file_content).unwrap();
                    assert_eq!(&file_content, content);
                }
                testcase.refresh();
                assert_eq!(testcase.mtime(), millis);
            }

            #[test]
            fn weather_dir() {
                // create the test cases
                let mut testcases = vec![
                    ("one.dat", testutil::generate_random_string(10)),
                    ("two.dat", testutil::generate_random_string(20)),
                    ("three.dat", testutil::generate_random_string(30)),
                ];
                testcases.sort_by(|lhs, rhs| lhs.0.cmp(rhs.0));
                let fixture = testutil::TestFixture::create();
                for (filename, content) in &testcases {
                    fixture.create_content(filename, content.as_bytes());
                }
                // spot check getting individual files
                let testcase = WeatherDir::new(&fixture.root_dir()).unwrap();
                for (filename, content) in &testcases {
                    let file = testcase.get_file(filename);
                    assert_eq!(&file.filename, filename);
                    assert_eq!(file.size(), content.len() as u64);
                }
                // spot check getting all files
                let mut all_files = testcase.get_files();
                assert_eq!(all_files.len(), testcases.len());
                all_files.sort_by(|lhs, rhs| lhs.filename.cmp(&rhs.filename));
                for i in 0..3 {
                    assert_eq!(all_files[i].filename, testcases[i].0);
                }
            }
        }
    }

    pub(crate) use archive::{to_daily_history, to_json, ArchiveData, ArchiveMd, WeatherArchive, WeatherHistory};
    mod archive {
        //! Support for weather data saved in `ZIP` archives.
        //!
        //! The implementation does not manage multi-client file access. That concern is left
        //! to the consummer of the module.
        use super::*;
        use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
        use serde_json as json;
        use std::{
            fs::OpenOptions,
            io::{Read, Write},
            time::SystemTime,
        };
        use toolslib::date_time::isodate;
        use zip::{read::ZipFile, write::FileOptions, DateTime, ZipArchive, ZipWriter};

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
        pub(crate) struct WeatherHistory(
            /// The managed weather archive.
            WeatherArchive,
        );
        impl WeatherHistory {
            /// Create a new instance of the weather archive manager.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location id.
            /// * `file` is the weather archive file.
            pub fn new(lid: &str, file: WeatherFile) -> Result<Self> {
                let archive = WeatherArchive::open(lid, file)?;
                Ok(Self(archive))
            }
            /// Right now only internal test builders use this.
            #[allow(unused)]
            pub fn lid(&self) -> &str {
                &self.0.lid
            }
            /// Creates a summary of the weather history statistics.
            pub fn summary(&self) -> Result<HistorySummary> {
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
                    location_id: self.0.lid.to_string(),
                    count: files,
                    overall_size: Some(self.0.file.size() as usize),
                    raw_size: Some(size as usize),
                    compressed_size: Some(compressed_size as usize),
                })
            }
            /// Get the weather history dates that are available.
            pub fn dates(&self) -> Result<DateRanges> {
                let iter = self.0.archive_iter(None, false, ArchiveMd::new)?;
                let dates: Vec<NaiveDate> = iter.map(|md| md.date).collect();
                Ok(DateRanges { location_id: self.0.lid.to_string(), date_ranges: DateRange::from_dates(dates) })
            }
            /// Get an iterator of daily weather history for a location.
            ///
            /// # Arguments
            ///
            /// * `filter` restricts the range of the historical weather data.
            ///
            pub fn daily_histories(&self, filter: &DateRange) -> Result<Vec<DailyHistory>> {
                let iter = self.0.archive_iter(Some(filter), true, daily_history_builder)?;
                let histories = iter.collect();
                Ok(histories)
            }
            /// Add weather history to the archive.
            ///
            /// # Arguments
            ///
            /// * `date` is the weather history date.
            /// * `data` is the parsed `JSON` document containing weather history.
            /// * `mtime` is the inernal archive file timestamp. If not provided the current time will be used.
            #[allow(unused)]
            pub fn add_data(&mut self, date: &NaiveDate, data: &json::Value, mtime: Option<i64>) -> Result<()> {
                let millis = match mtime {
                    Some(mtime) => mtime,
                    None => SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64,
                };
                self.0.add_data(date, data, millis)
            }
        }

        fn daily_history_builder(lid: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<DailyHistory> {
            let data = ArchiveData::new(lid, date, zipfile)?;
            let json = data.json()?;
            json_to_daily_history(lid, date.clone(), &json)
        }

        /// The manager for a `Zip` archive with weather data.
        #[derive(Debug)]
        pub(crate) struct WeatherArchive {
            /// The unique identifier for a location.
            lid: String,
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
            /// * `lid` is the location identifier.
            /// * `file` is the archive containing of weather data.
            pub fn open(lid: &str, file: WeatherFile) -> Result<Self> {
                if !file.exists() {
                    Err(archive_err!(lid, format!("'{}' does not exist...", &file)))
                } else {
                    match ZipArchive::new(file.reader()?) {
                        // unfortunately you have to drop the zip archive which makes open/create expensive
                        Ok(_) => Ok(Self { lid: lid.to_string(), file }),
                        Err(error) => Err(archive_err!(lid, &error)),
                    }
                }
            }
            /// Creates a new weather data archive and the manager for it
            ///
            /// An error will be returned if the archive exists or there are problems trying to create it.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location identifier.
            /// * `weather_file` is the container of weather data.
            #[allow(unused)]
            fn create(lid: &str, mut file: WeatherFile) -> Result<Self> {
                if file.exists() {
                    Err(archive_err!(&lid, format!("'{}' already exists...", &file)))
                } else {
                    // touch the file so the writer can be returned.
                    if let Err(err) = OpenOptions::new().create(true).write(true).open(&file.to_string()) {
                        Err(archive_err!(lid, &format!("Error creating archive file ({}), {}", &file, &err)))
                    } else {
                        let writer = file.writer()?;
                        let mut archive = ZipWriter::new(writer);
                        match archive.finish() {
                            Ok(_) => {
                                file.refresh();
                                Self::open(lid, file)
                            }
                            Err(err) => Err(archive_err!(lid, &err)),
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
            pub fn archive_iter<T>(
                &self,
                filter: Option<&DateRange>,
                sort: bool,
                builder: HistoryBuilder<T>,
            ) -> Result<ArchiveIter<T>> {
                let mut reader = self.get_reader()?;
                let mut history_dates = Self::filter_history(&mut reader, filter);
                if sort {
                    history_dates.sort()
                }
                Ok(ArchiveIter::new(&self.lid, reader, history_dates, builder))
            }
            /// Add weather data history to the archive.
            ///
            /// # Arguments
            ///
            /// * `date` is the weather history date.
            /// * `json` is the historical weather data.
            /// * `mtime` is the internal archive file timestamp.
            fn add_data(&mut self, date: &NaiveDate, data: &json::Value, mtime: i64) -> Result<()> {
                let mut writer = ArchiveWriter::new(&self.lid, self.get_writer()?);
                writer.add_json(date, data, mtime)
            }
            /// This is used internally right now to build test case data.
            #[allow(unused)]
            fn add_bulk<I>(&mut self, data_collection: I) -> Result<u64>
            where
                I: Iterator<Item = (NaiveDate, Vec<u8>, i64)>,
            {
                let mut writer = ArchiveWriter::new(&self.lid, self.get_writer()?);
                let mut written = 0;
                for (date, data, mtime) in data_collection {
                    writer.add_data(&date, &data, mtime)?;
                    written += 1;
                }
                Ok(written)
            }
            /// Create the manager that writes content to the archive.
            fn get_writer(&self) -> Result<ZipWriter<File>> {
                match self.file.writer() {
                    Ok(file_writer) => match ZipWriter::new_append(file_writer) {
                        Ok(zip_writer) => Ok(zip_writer),
                        Err(err) => {
                            let reason = format!("'{}' zip writer error ({}).", self.file.filename, &err);
                            Err(archive_err!(&self.lid, reason))
                        }
                    },
                    Err(err) => {
                        let reason = format!("'{}' file writer error ({}).", self.file.filename, &err);
                        Err(archive_err!(&self.lid, reason))
                    }
                }
            }
            /// Create the manager that reads content from the archive.
            fn get_reader(&self) -> Result<ZipArchiveReader> {
                match ZipArchive::new(self.file.reader()?) {
                    Ok(reader) => Ok(reader),
                    Err(err) => Err(archive_err!(&self.lid, &format!("get_reader error ({}).", &err))),
                }
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
                reader
                    .file_names()
                    .filter_map(|history_name| match WeatherArchive::filename_to_date(history_name) {
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
                    .collect()
            }
            /// Build the internal archive filename to the provided date.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location id.
            /// * `date` is the history date that will be embedded into the filename.
            fn date_to_filename(lid: &str, date: &NaiveDate) -> String {
                format!("{}/{}-{}.json", lid, lid, toolslib::date_time::fmt_date(date, "%Y%m%d"))
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
            pub lid: String,
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
            /// * `lid` is the location identifier.
            /// * `date` is the date associated with the history file.
            /// * `zipfile` provides access to the history file metrics.
            pub fn new(lid: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<Self> {
                let mtime = Self::datetime_to_millis(lid, zipfile.last_modified());
                Ok(Self {
                    lid: lid.to_string(),
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
            /// * `lid` is the location identifier.
            /// * `datetime` is the `ZIP` file timestamp.
            fn datetime_to_millis(lid: &str, datetime: DateTime) -> i64 {
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
                            log::error!("{}", &archive_err!(lid, reason));
                            0
                        }
                    } else {
                        let reason = format!("NaiveDate error for YYYYMMDD ({:04}{:02}{:02})", year, month, day);
                        log::error!("{}", &archive_err!(lid, reason));
                        0
                    }
                }
            }
        }

        /// A bean providing the contents of a weather history file in the archive.
        #[derive(Debug)]
        pub struct ArchiveData {
            /// The location identifier.
            pub lid: String,
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
            /// * `lid` is the location identifier.
            /// * `date` is the date of the weather history.
            /// * `zipfile` provides the contents of the history file in the archive.
            pub fn new(lid: &str, date: &NaiveDate, zipfile: ZipFile) -> Result<Self> {
                let size = zipfile.size() as usize;
                let mut data = Vec::with_capacity(size);
                for result in zipfile.bytes() {
                    match result {
                        Ok(b) => data.push(b),
                        Err(err) => {
                            let reason = format!("{} read error at byte {} ({}).", lid, data.len(), &err);
                            log::error!("{}", &archive_err!(lid, reason));
                            break;
                        }
                    }
                }
                match data.len() == size {
                    true => Ok(Self { lid: lid.to_string(), date: date.clone(), data }),
                    false => {
                        let reason = format!("expected {} bytes and only read {}.", size, data.len());
                        Err(archive_err!(lid, reason))
                    }
                }
            }
            /// Get the file contents as a slice of bytes.
            fn bytes(&self) -> &[u8] {
                &self.data
            }
            /// Get the file contents as a parsed `JSON` document.
            pub fn json(&self) -> Result<json::Value> {
                // let reader_result: std::result::Result<json::Value, json::Error> = json::from_reader(self.bytes());
                // match reader_result {
                //     Ok(value) => Ok(value),
                //     Err(err) => {
                //         let reason = format!("{} to JSON error ({})", isodate(&self.date), &err);
                //         Err(archive_err!(&self.lid, reason))
                //     }
                // }
                match to_json(self.bytes()) {
                    Ok(json) => Ok(json),
                    Err(err) => {
                        let reason = format!("{} to JSON error ({})", isodate(&self.date), &err);
                        Err(archive_err!(&self.lid, reason))
                    }
                }
            }
        }

        pub(crate) fn to_json(bytes: &[u8]) -> Result<json::Value> {
            let reader_result: std::result::Result<json::Value, json::Error> = json::from_reader(bytes);
            match reader_result {
                Ok(value) => Ok(value),
                Err(err) => Err(Error::from(err.to_string())),
            }
        }

        /// Convert the Dark Sky document to daily weather history.
        ///
        /// # Arguments
        ///
        /// * `lid` is the location identifier.
        /// * `date` is the date of the weather history.
        /// * `json` contains the parsed weather history document.
        fn json_to_daily_history(lid: &str, date: NaiveDate, json: &json::Value) -> Result<DailyHistory> {
            to_daily_history(lid, date, &json["daily"]["data"][0])
            // let daily = &json["daily"]["data"][0];
            // if daily.is_object() {
            //     let daily_history = DailyHistory {
            //         location_id: lid.to_string(),
            //         date,
            //         temperature_high: daily.get("temperatureHigh").map_or(None, |v| v.as_f64()),
            //         temperature_high_time: daily.get("temperatureHighTime").map_or(None, |v| v.as_i64()),
            //         temperature_low: daily.get("temperatureLow").map_or(None, |v| v.as_f64()),
            //         temperature_low_time: daily.get("temperatureLowTime").map_or(None, |v| v.as_i64()),
            //         temperature_max: daily.get("temperatureMax").map_or(None, |v| v.as_f64()),
            //         temperature_max_time: daily.get("temperatureMaxTime").map_or(None, |v| v.as_i64()),
            //         temperature_min: daily.get("temperatureMin").map_or(None, |v| v.as_f64()),
            //         temperature_min_time: daily.get("temperatureMinTime").map_or(None, |v| v.as_i64()),
            //         wind_speed: daily.get("windSpeed").map_or(None, |v| v.as_f64()),
            //         wind_gust: daily.get("windGust").map_or(None, |v| v.as_f64()),
            //         wind_gust_time: daily.get("windGustTime").map_or(None, |v| v.as_i64()),
            //         wind_bearing: daily.get("windBearing").map_or(None, |v| v.as_i64()),
            //         cloud_cover: daily.get("cloudCover").map_or(None, |v| v.as_f64()),
            //         uv_index: daily.get("uvIndex").map_or(None, |v| v.as_i64()),
            //         uv_index_time: daily.get("uvIndexTime").map_or(None, |v| v.as_i64()),
            //         summary: daily.get("summary").map_or(None, |v| match v {
            //             json::Value::Null => None,
            //             json::Value::Number(n) => Some(n.to_string()),
            //             _ => Some(v.as_str().map_or(String::default(), |s| s.to_string())),
            //         }),
            //         humidity: daily.get("humidity").map_or(None, |v| v.as_f64()),
            //         dew_point: daily.get("dewPoint").map_or(None, |v| v.as_f64()),
            //         sunrise_time: daily.get("sunriseTime").map_or(None, |v| v.as_i64()),
            //         sunset_time: daily.get("sunsetTime").map_or(None, |v| v.as_i64()),
            //         moon_phase: daily.get("moonPhase").map_or(None, |v| v.as_f64()),
            //     };
            //     // Some(daily_history)
            //     Ok(daily_history)
            // } else {
            //     let reason = format!("Did not find daily history for {}", isodate(&date));
            //     Err(archive_err!(lid, reason))
            // }
        }

        pub(crate) fn to_daily_history(lid: &str, date: NaiveDate, json: &json::Value) -> Result<DailyHistory> {
            if json.is_object() {
                let daily_history = DailyHistory {
                    location_id: lid.to_string(),
                    date,
                    temperature_high: json.get("temperatureHigh").map_or(None, |v| v.as_f64()),
                    temperature_high_time: json.get("temperatureHighTime").map_or(None, |v| v.as_i64()),
                    temperature_low: json.get("temperatureLow").map_or(None, |v| v.as_f64()),
                    temperature_low_time: json.get("temperatureLowTime").map_or(None, |v| v.as_i64()),
                    temperature_max: json.get("temperatureMax").map_or(None, |v| v.as_f64()),
                    temperature_max_time: json.get("temperatureMaxTime").map_or(None, |v| v.as_i64()),
                    temperature_min: json.get("temperatureMin").map_or(None, |v| v.as_f64()),
                    temperature_min_time: json.get("temperatureMinTime").map_or(None, |v| v.as_i64()),
                    wind_speed: json.get("windSpeed").map_or(None, |v| v.as_f64()),
                    wind_gust: json.get("windGust").map_or(None, |v| v.as_f64()),
                    wind_gust_time: json.get("windGustTime").map_or(None, |v| v.as_i64()),
                    wind_bearing: json.get("windBearing").map_or(None, |v| v.as_i64()),
                    cloud_cover: json.get("cloudCover").map_or(None, |v| v.as_f64()),
                    uv_index: json.get("uvIndex").map_or(None, |v| v.as_i64()),
                    uv_index_time: json.get("uvIndexTime").map_or(None, |v| v.as_i64()),
                    summary: json.get("summary").map_or(None, |v| match v {
                        json::Value::Null => None,
                        json::Value::Number(n) => Some(n.to_string()),
                        _ => Some(v.as_str().map_or(String::default(), |s| s.to_string())),
                    }),
                    humidity: json.get("humidity").map_or(None, |v| v.as_f64()),
                    dew_point: json.get("dewPoint").map_or(None, |v| v.as_f64()),
                    sunrise_time: json.get("sunriseTime").map_or(None, |v| v.as_i64()),
                    sunset_time: json.get("sunsetTime").map_or(None, |v| v.as_i64()),
                    moon_phase: json.get("moonPhase").map_or(None, |v| v.as_f64()),
                };
                // Some(daily_history)
                Ok(daily_history)
            } else {
                let reason = format!("Did not find daily history for {}", isodate(&date));
                Err(archive_err!(lid, reason))
            }
        }

        /// The function signature used by the weather archive iterator to create history data.
        type HistoryBuilder<T> = fn(&str, &NaiveDate, ZipFile) -> Result<T>;

        /// The low-level iterator over weather history in the archive.
        pub struct ArchiveIter<T> {
            /// The location identifier.
            lid: String,
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
            /// * `lid` is the location identifier.
            /// * `reader` is the `ZIP` archive reader.
            /// * `dates` identify what weather history will be returned.
            /// * `make` is what the iterator uses to return the history data.
            fn new(lid: &str, reader: ZipArchiveReader, dates: Vec<NaiveDate>, make: HistoryBuilder<T>) -> Self {
                Self { lid: lid.to_string(), reader, dates, index: 0, make }
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
                    let history_name = WeatherArchive::date_to_filename(&self.lid, &date);
                    match self.reader.by_name(&history_name) {
                        Ok(zip_file) => match (self.make)(&self.lid, &date, zip_file) {
                            Ok(data) => {
                                history.replace(data);
                            }
                            Err(err) => {
                                let reason = format!("HistoryBuilder error ({})", &err);
                                log::error!("{}", &archive_err!(&self.lid, reason));
                            }
                        },
                        Err(err) => {
                            let reason = format!("by_name error ({})", &err);
                            log::error!("{}", &archive_err!(&self.lid, reason));
                        }
                    }
                }
                history
            }
        }

        /// Create the manager that writes weather data history to the archive.
        struct ArchiveWriter {
            /// The location identifier.
            lid: String,
            /// The `ZIP` file writer.
            writer: ZipWriter<File>,
        }
        impl ArchiveWriter {
            /// Create a new instance of the weather writer.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location identifier.
            /// * `writer` is the `ZIP` file writer.
            fn new(lid: &str, writer: ZipWriter<File>) -> Self {
                Self { lid: lid.to_string(), writer }
            }
            /// Add weather data to the archive.
            ///
            /// # Arguments
            ///
            /// * `date` is the date of the weather history.
            /// * `data` is the weather history data.
            /// * `mtime` is the last modified time of the weather history.
            fn add_data(&mut self, date: &NaiveDate, data: &[u8], mtime: i64) -> Result<()> {
                let mod_time = Self::millis_to_datetime(&self.lid, mtime);
                let filename = WeatherArchive::date_to_filename(&self.lid, &date);
                let options = FileOptions::default().last_modified_time(mod_time);
                if let Err(err) = self.writer.start_file(filename, options) {
                    let reason = format!("{} write start_file err ({}).", date, &err);
                    Err(archive_err!(&self.lid, reason))
                } else if let Err(err) = self.writer.write_all(data) {
                    let reason = format!("{} write start_all err ({}).", date, &err);
                    Err(archive_err!(&self.lid, reason))
                } else {
                    Ok(())
                }
            }
            /// Add weather data to the archive.
            ///
            /// # Arguments
            ///
            /// * `date` is the date of the weather history.
            /// * `json` is the weather history data.
            /// * `mtime` is the last modified time of the weather history.
            fn add_json(&mut self, date: &NaiveDate, data: &json::Value, mtime: i64) -> Result<()> {
                let vec_result: std::result::Result<Vec<u8>, json::Error> = json::to_vec(data);
                match vec_result {
                    Ok(data) => self.add_data(date, &data, mtime),
                    Err(err) => {
                        let reason = format!("{} from JSON error ({})", isodate(&date), &err);
                        Err(archive_err!(&self.lid, reason))
                    }
                }
            }
            /// Convert milliseconds to a `ZIP` date time.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location identifier.
            /// * `millis` is the timestamp in milliseconds.
            fn millis_to_datetime(lid: &str, millis: i64) -> DateTime {
                match NaiveDateTime::from_timestamp_millis(millis) {
                    Some(naive_datetime) => match naive_datetime.year() {
                        year if year < 1980 || year > 2107 => {
                            let err = archive_err!(lid, format!("illegal year '{}'", year));
                            log::error!("{}", &err);
                            DateTime::default()
                        }
                        year => {
                            let month = naive_datetime.month() as u8;
                            let day = naive_datetime.day() as u8;
                            let hour = naive_datetime.hour() as u8;
                            let minute = naive_datetime.minute() as u8;
                            let second = naive_datetime.second() as u8;
                            // it should be safe to ignore the result since the bounds are checked
                            DateTime::from_date_and_time(year as u16, month, day, hour, minute, second).unwrap()
                        }
                    },
                    None => {
                        let err = archive_err!(lid, format!("NaiveDateTime error {}ms", millis));
                        log::error!("{}", &err);
                        DateTime::default()
                    }
                }
            }
        }

        #[cfg(test)]
        // use super::super::tests_weather_dir as test_resources_weather_dir;
        #[cfg(test)]
        mod test {
            use super::file::WeatherDir;
            use super::*;
            use toolslib::date_time::{get_date, isodate};

            #[test]
            fn create_open() {
                let fixture = testutil::TestFixture::create();
                let weather_dir = WeatherDir::new(&fixture.root_dir()).unwrap();
                let lid = "init";
                macro_rules! get_file {
                    () => {
                        // weather_dir.get_file(&test_archive!(lid))
                        weather_dir.get_file(&archive_name(lid))
                    };
                }
                assert!(!get_file!().exists());
                assert!(WeatherArchive::open(lid, get_file!()).is_err());
                assert!(WeatherArchive::create(lid, get_file!()).is_ok());
                assert!(get_file!().exists());
                assert!(WeatherArchive::open(lid, get_file!()).is_ok());
                assert!(WeatherArchive::create(lid, get_file!()).is_err());
            }

            #[test]
            fn weather_data_iterator() {
                let weather_dir = testutil::test_resources_weather_dir().unwrap();
                let lid = "testmd";
                let file = weather_dir.get_file(&&archive_name(lid));
                assert!(file.exists());
                let from = get_date(2014, 4, 1);
                let to = get_date(2014, 4, 5);
                let history_range = DateRange::new(from.clone(), to.clone());
                let archive = WeatherArchive::open(lid, file).unwrap();
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
                let lid = "testmd";
                let weather_dir = testutil::test_resources_weather_dir().unwrap();
                let file = weather_dir.get_file(&archive_name(lid));
                let weather_history = WeatherHistory::new(lid, file).unwrap();
                let testcase = weather_history.summary().unwrap();
                assert_eq!(testcase.location_id, lid);
                assert_eq!(testcase.count, 28);
                assert_eq!(testcase.overall_size, Some(43172));
                assert_eq!(testcase.raw_size, Some(263500));
                assert_eq!(testcase.compressed_size, Some(39510));
            }

            #[test]
            fn datetime_convert() {
                // lower bounds
                let ts = NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1980, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                )
                .timestamp_millis();
                let expected = DateTime::default();
                let testcase = ArchiveWriter::millis_to_datetime("test", ts);
                assert_eq!(testcase.datepart(), expected.datepart());
                assert_eq!(testcase.timepart(), expected.timepart());
                // upper bounds
                let ts = NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2107, 12, 31).unwrap(),
                    NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
                )
                .timestamp_millis();
                let expected = DateTime::from_date_and_time(2107, 12, 31, 23, 59, 59).unwrap();
                let testcase = ArchiveWriter::millis_to_datetime("test", ts);
                assert_eq!(testcase.datepart(), expected.datepart());
                assert_eq!(testcase.timepart(), expected.timepart());
                let testcase = ArchiveMd::datetime_to_millis("test", testcase);
                assert_eq!(testcase, ts);
                // upper out of bounds
                let ts = NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2108, 1, 1).unwrap(),
                    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                )
                .timestamp_millis();
                let expected = DateTime::default();
                let testcase = ArchiveWriter::millis_to_datetime("test", ts);
                assert_eq!(testcase.datepart(), expected.datepart());
                assert_eq!(testcase.timepart(), expected.timepart());
            }

            #[test]
            fn writer() {
                // create the test archive
                // let weather_dir = testutil::test_resources_weather_dir().unwrap();
                let test_fixture = testutil::TestFixture::create();
                let weather_dir = WeatherDir::new(&test_fixture.root_dir()).unwrap();
                let lid = "writertest";
                let filename = archive_name(lid);
                let mut file = weather_dir.get_file(&filename);
                // JIC it's not a test fixture diretory
                if file.exists() {
                    match std::fs::remove_file(&file.to_string()) {
                        Ok(_) => file.refresh(),
                        Err(err) => eprintln!("{}: {}", &file, &err),
                    }
                }
                let mut testcase = WeatherArchive::create(lid, file).unwrap();

                // test helpers
                fn mk_data(date: &NaiveDate) -> String {
                    format!(r#"{{"date":"{}"}}"#, isodate(date))
                }
                fn mk_mtime(date: &NaiveDate, hour: usize) -> i64 {
                    let time = NaiveTime::from_hms_opt(hour as u32, 0, 0).unwrap();
                    let mtime_ts = NaiveDateTime::new(date.clone(), time);
                    (mtime_ts - NaiveDateTime::default()).num_milliseconds()
                }

                // write some JSON to the archive
                let history_range = DateRange::new(get_date(2023, 7, 1), get_date(2023, 7, 4));
                let history_dates = history_range.as_iter().collect::<Vec<NaiveDate>>();
                for (hour, date) in history_dates.iter().enumerate() {
                    let data = mk_data(date);
                    let value: json::Value = json::from_str(&data).unwrap();
                    let mtime = mk_mtime(date, hour);
                    testcase.add_data(date, &value, mtime).unwrap();
                }

                // now spot check the archive metadata
                let date_iter = history_dates.iter();
                // let md_iter = testcase.metadata(Some(&history_range), true).unwrap();
                let md_iter = testcase.archive_iter(Some(&history_range), true, ArchiveMd::new).unwrap();
                for (hour, (date, archive_md)) in std::iter::zip(date_iter, md_iter).enumerate() {
                    assert_eq!(date, &archive_md.date);
                    assert_eq!(mk_mtime(date, hour), archive_md.mtime);
                }

                // now spot check the archive content
                let date_iter = history_dates.iter();
                let data_iter = testcase.archive_iter(Some(&history_range), true, ArchiveData::new).unwrap();
                for (date, data) in std::iter::zip(date_iter, data_iter) {
                    let json = data.json().unwrap().to_string();
                    assert_eq!(json, mk_data(date));
                }
            }

            #[allow(unused)]
            const SOURCE_WEATHER_DATA: &str = r"C:\Users\rncru\dev\weather_data";

            // create the metadata test archive
            // #[test]
            #[allow(unused)]
            fn create_test_metadata_archive() {
                // setup the test archive
                let to_lid = "testmd";
                let to_dir = testutil::test_resources_weather_dir().unwrap();
                let mut to_file = to_dir.get_file(&archive_name(to_lid));
                if to_file.exists() {
                    eprintln!("removing {}", archive_name(to_lid));
                    std::fs::remove_file(&to_file.to_string()).unwrap();
                    to_file.refresh();
                }
                let mut to = WeatherArchive::create(to_lid, to_file).unwrap();
                // setup the source archive
                let from_lid = "tigard";
                let from_dir = WeatherDir::new(SOURCE_WEATHER_DATA).unwrap();
                let from_file = from_dir.get_file(&archive_name(from_lid));
                let from = WeatherArchive::open(from_lid, from_file).unwrap();
                // now copy the test data
                let histories = vec![
                    DateRange::new(get_date(2014, 4, 1), get_date(2014, 4, 7)),
                    DateRange::new(get_date(2015, 5, 8), get_date(2015, 5, 14)),
                    DateRange::new(get_date(2016, 6, 15), get_date(2016, 6, 21)),
                    DateRange::new(get_date(2017, 7, 22), get_date(2017, 7, 28)),
                ];
                for history_range in &histories {
                    let mds = from.archive_iter(Some(history_range), true, ArchiveMd::new).unwrap();
                    let histories = from.archive_iter(Some(history_range), true, ArchiveData::new).unwrap();
                    let bulk: Vec<(NaiveDate, Vec<u8>, i64)> = std::iter::zip(mds, histories)
                        .map(|(md, history)| {
                            assert_eq!(md.date, history.date);
                            (md.date, history.data, md.mtime)
                        })
                        .collect();
                    to.add_bulk(bulk.into_iter()).unwrap();
                }
            }

            #[allow(unused)]
            // #[test]
            fn create_test_archives() {
                // the from archive to test archive mappings
                let from_to = vec![("tigard", "north"), ("carson_city_nv", "between"), ("tucson", "south")];
                // the directory helpers
                let from_dir = WeatherDir::new(SOURCE_WEATHER_DATA).unwrap();
                let dst_dir = testutil::test_resources_weather_dir().unwrap();
                // make sure the destinations are pristine
                from_to.iter().for_each(|(_, to_id)| {
                    let dst_path = PathBuf::from(&dst_dir.to_string()).join(&archive_name(to_id));
                    if dst_path.exists() {
                        eprintln!("removing test resource: {:?}", dst_path);
                        std::fs::remove_file(&dst_path).unwrap();
                    }
                    WeatherArchive::create(to_id, dst_dir.get_file(&archive_name(to_id))).unwrap();
                });
                // these are the history ranges to mine from the real weather data
                let history_dates = vec![
                    DateRange::new(get_date(2015, 4, 1), get_date(2015, 4, 14)),
                    DateRange::new(get_date(2016, 10, 10), get_date(2016, 10, 17)),
                    DateRange::new(get_date(2017, 7, 14), get_date(2017, 7, 20)),
                    DateRange::new(get_date(2018, 1, 1), get_date(2018, 1, 7)),
                ];
                // walk the history ranges and mine the history
                history_dates.iter().for_each(|history_range| {
                    from_to.iter().for_each(|(from_id, to_id)| {
                        let from_file = from_dir.get_file(&archive_name(from_id));
                        let mut from_archive = ZipArchive::new(from_file.reader().unwrap()).unwrap();
                        let to_file = dst_dir.get_file(&archive_name(to_id));
                        let mut to_archive = ZipWriter::new_append(to_file.writer().unwrap()).unwrap();
                        history_range.as_iter().for_each(|date| {
                            let from_filename = WeatherArchive::date_to_filename(from_id, &date);
                            match from_archive.by_name(&from_filename) {
                                Ok(file) => {
                                    let to_filename = WeatherArchive::date_to_filename(to_id, &date);
                                    to_archive.raw_copy_file_rename(file, &to_filename).unwrap();
                                }
                                Err(err) => match err {
                                    zip::result::ZipError::FileNotFound => {
                                        eprintln!("{} not found...", from_filename)
                                    }
                                    _ => panic!("error getting {}: {}", from_filename, err.to_string()),
                                },
                            }
                        })
                    });
                });
            }
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

        /// The [WeatherLocations] error builder.
        macro_rules! locations_err {
            ($reason:expr) => {
                Error::from(format!("WeatherLocations: {}", $reason))
            };
        }

        pub(crate) fn create(weather_dir: &WeatherDir) -> Result<WeatherLocations> {
            let file = weather_dir.get_file(LOCATIONS_FILENAME);
            if file.exists() {
                WeatherLocations::new(file)
            } else {
                log::warn!("{} does not exist", file);
                Ok(WeatherLocations(vec![]))
            }
        }

        /// The locations `JSON` document manager.
        #[derive(Debug)]
        pub(crate) struct WeatherLocations(
            /// The locations metadata.
            Vec<LocationMd>,
        );
        impl WeatherLocations {
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
            /// * `case_sensitive` will make filters case sensitive (`true`) or ignore case (`false`).
            /// * `sort` will order the matching locations by their name.
            pub fn as_iter(&self, patterns: &Vec<String>, case_sensitive: bool, sort: bool) -> LocationsIter {
                let prepare = |text: &str| if case_sensitive { text.to_string() } else { text.to_lowercase() };
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
                    id: md.alias.to_lowercase(),
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
                let weather_dir = testutil::test_resources_weather_dir().unwrap();
                let testcase = WeatherLocations::new(weather_dir.get_file("locations.json")).unwrap();
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
                assert_eq!(testcase.id, md.alias.to_lowercase());
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

        use super::{archive_name, weather_dir, weather_locations, WeatherDir, WeatherHistory};
        use crate::{
            backend::{DataAdapter, Error, Result},
            prelude::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location},
        };

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

        /// The archive implemenation of a [DataAdapter].
        pub(crate) struct ArchiveDataAdapter(
            /// The directory containing weather data files
            WeatherDir,
        );
        impl ArchiveDataAdapter {
            /// Used internally to get the archive manager for some location.
            ///
            /// # Arguments
            ///
            /// * `lid` is the location identifier.
            fn get_archive(&self, lid: &str) -> Result<WeatherHistory> {
                let weather_file = self.0.get_file(&archive_name(lid));
                let weather_history = WeatherHistory::new(lid, weather_file)?;
                Ok(weather_history)
            }
        }
        impl DataAdapter for ArchiveDataAdapter {
            /// Returns the daily weather data history for a location.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies what location should be used.
            /// * `history_range` specifies the date range that should be used.
            fn daily_histories(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories> {
                let mut locations = self.locations(criteria)?;
                match locations.len() {
                    1 => {
                        let location = locations.pop().unwrap();
                        let archive = self.get_archive(&location.id)?;
                        let daily_histories = archive.daily_histories(&history_range)?;
                        Ok(DailyHistories { location, daily_histories })
                    }
                    0 => Err(Error::from("A location was not found.")),
                    _ => Err(Error::from("Multiple locations were found.")),
                }
            }
            /// Get the weather history dates for locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations.
            fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>> {
                let locations = self.locations(criteria)?;
                let mut history_dates = Vec::with_capacity(locations.len());
                for location in locations {
                    let archive = self.get_archive(&location.id)?;
                    let dates = archive.dates()?;
                    history_dates.push(HistoryDates { location, history_dates: dates.date_ranges })
                }
                Ok(history_dates)
            }
            /// Get the summary metrics of a locations weather data.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations that should be used.
            fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>> {
                let locations = self.locations(criteria)?;
                let mut history_summaries = Vec::with_capacity(locations.len());
                for location in locations {
                    let archive = self.get_archive(&location.id)?;
                    let summary = archive.summary()?;
                    history_summaries.push(HistorySummaries {
                        location,
                        count: summary.count,
                        overall_size: summary.overall_size,
                        raw_size: summary.raw_size,
                        compressed_size: summary.compressed_size,
                    });
                }
                Ok(history_summaries)
            }
            /// Get the metadata for weather locations.
            ///
            /// # Arguments
            ///
            /// * `criteria` identifies the locations of interest.
            fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>> {
                let weather_locations = weather_locations(&self.0)?;
                let locations = weather_locations.as_iter(&criteria.filters, !criteria.icase, criteria.sort).collect();
                Ok(locations)
            }
        }
    }

    #[cfg(test)]
    mod testutil {
        //! Collect all the unit test utilites here for now
        use super::{file::WeatherDir, Result};
        use crate::backend::testlib::test_resources;
        use filetime::{set_file_mtime, FileTime};
        use rand::Rng;
        use std::{env, fs, io::Write, path::PathBuf, time::SystemTime};

        /// Used to create a temporary weather directory and delete it as part of the function exit.
        #[derive(Debug)]
        pub struct TestFixture {
            root_dir: PathBuf,
        }
        impl TestFixture {
            /// Creates a test weather directory or panics if a unique directory cannot be created.
            pub fn create() -> Self {
                let tmpdir = env::temp_dir();
                let mut weather_dir: Option<PathBuf> = None;
                // try to create a test directory 10 times
                for _ in [0..10] {
                    let test_dir = tmpdir.join(format!("weather_dir-{}", generate_random_string(15)));
                    match test_dir.exists() {
                        true => {
                            eprintln!("Test directory '{}' exists...", test_dir.as_path().display())
                        }
                        false => {
                            weather_dir.replace(test_dir);
                            break;
                        }
                    }
                }
                match weather_dir {
                    Some(root_dir) => match fs::create_dir(&root_dir) {
                        Ok(_) => Self { root_dir },
                        Err(e) => {
                            panic!("Error creating '{}': {}", root_dir.as_path().display(), e.to_string())
                        }
                    },
                    None => panic!("Tried 10 times to get a unique test directory name and failed..."),
                }
            }
            pub fn root_dir(&self) -> String {
                self.root_dir.as_path().display().to_string()
            }
            pub fn create_path(&self, filename: &str) -> PathBuf {
                self.root_dir.join(filename)
            }
            pub fn create_content(&self, filename: &str, data: &[u8]) {
                let path = self.create_path(filename);
                let mut file = if path.exists() {
                    match fs::OpenOptions::new().write(true).open(&path) {
                        Ok(f) => f,
                        Err(e) => panic!("Error opening '{}': {}", path.display(), e.to_string()),
                    }
                } else {
                    fs::File::create(&path).expect(format!("Error creating {}", path.display()).as_str())
                };
                if let Err(error) = file.write_all(data) {
                    panic!("Error writing '{}' content: {}", path.display(), error.to_string());
                }
                file.sync_all().unwrap();
            }
            pub fn set_modtime(&self, filename: &str, time: &SystemTime) {
                let path = self.create_path(filename);
                if let Err(error) = set_file_mtime(&path, FileTime::from_system_time(time.clone())) {
                    panic!("Error setting '{}' modtime: {}", path.display(), error.to_string());
                }
            }
        }
        impl Drop for TestFixture {
            /// Clean up the temporary directory as best you can.
            fn drop(&mut self) {
                if let Err(e) = fs::remove_dir_all(self.root_dir.as_path()) {
                    eprintln!("Yikes... Error cleaning up test weather_dir: {}", e.to_string());
                }
            }
        }

        pub(super) fn test_resources_weather_dir() -> Result<WeatherDir> {
            let path = test_resources().join("filesys");
            WeatherDir::new(&path.as_path().display().to_string())
        }

        pub(super) fn generate_random_string(len: usize) -> String {
            let mut rand = rand::thread_rng();
            const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmonopqrstuvwxyz0123456789";
            // let unique_sequence: String = (0..15)
            let random_string = (0..len)
                .map(|_| {
                    let idx = rand.gen_range(0..CHARS.len());
                    CHARS[idx] as char
                })
                .collect();
            // eprintln!("generate_random_string: {}...", random_string);
            random_string
        }
    }
}
