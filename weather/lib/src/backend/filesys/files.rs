//! Support for filesystem access.
//!
use super::*;

pub use v2::{WeatherDir, WeatherFile};
mod v2 {
    //! The current implementation of filesystem access.
    use super::*;

    use std::{
        fs::{remove_file, rename, File, Metadata, OpenOptions},
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
    pub struct WeatherDir(
        /// The directory managed by the weather directory.
        PathBuf,
    );

    impl std::fmt::Display for WeatherDir {
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

    impl TryFrom<&Config> for WeatherDir {
        type Error = Error;
        fn try_from(config: &Config) -> std::prelude::v1::Result<Self, Self::Error> {
            WeatherDir::new(PathBuf::from(&config.weather_data.directory))
        }
    }

    impl WeatherDir {
        /// Creates a new instance of the weather directory manager.
        ///
        /// An error will be returned if the directory does not exist, or does exist but is not a directory.
        ///
        /// # Arguments
        ///
        /// * `directory_name` is the name of the directory.
        pub fn new(path: PathBuf) -> Result<WeatherDir> {
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
    pub struct WeatherFile {
        /// The file name within the weather directory.
        pub filename: String,
        /// The file path.
        path: PathBuf,
        /// The filesystem metadata for the file.
        fs_metadata: Option<Metadata>,
    }
    impl std::fmt::Display for WeatherFile {
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
        pub fn refresh(&mut self) {
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
        pub fn exists(&self) -> bool {
            self.fs_metadata.is_some()
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
                Err(err) => Err(file_err!(&self.filename, &format!("open read/write error ({}).", &err))),
            }
        }
        /// Get the reader that can be used to read the contents of a Zip archive.
        pub fn reader(&self) -> Result<File> {
            match OpenOptions::new().read(true).open(&self.path) {
                Ok(file) => Ok(file),
                Err(err) => Err(file_err!(&self.filename, &format!("open read error ({})...", &err))),
            }
        }
        /// Get the weather file as a [Path].
        pub fn path(&self) -> &Path {
            self.path.as_path()
        }
        /// Remove the weather file from the filesystem.
        pub fn remove(&mut self) -> Result<()> {
            self.refresh();
            match self.exists() {
                true => match remove_file(self.path()) {
                    Ok(_) => {
                        self.refresh();
                        Ok(())
                    }
                    Err(err) => {
                        let error = file_err!(self.filename, format!("Error removing file ({}).", err));
                        log::error!("{}", error);
                        Err(error)
                    }
                },
                false => {
                    log::trace!("{}", file_err!(self.filename, "Does not exist..."));
                    Ok(())
                }
            }
        }
        /// Either update the existing file access time or create the file.
        pub fn touch(&mut self) -> Result<()> {
            self.refresh();
            let result = if self.exists() {
                OpenOptions::new().read(true).open(self.path())
            } else {
                OpenOptions::new().write(true).create(true).open(self.path())
            };
            match result {
                Ok(_) => {
                    self.refresh();
                    Ok(())
                }
                Err(err) => {
                    let error = file_err!(self.filename, format!("Error touching file ({}).", err));
                    log::error!("{}", error);
                    Err(error)
                }
            }
        }
        /// Rename the weather file to another weather file.
        pub fn rename(&mut self, to: &mut WeatherFile) -> Result<()> {
            match rename(self.path(), to.path()) {
                Ok(_) => {
                    self.refresh();
                    to.refresh();
                    Ok(())
                }
                Err(err) => {
                    let error = file_err!(self.filename, format!("Error renaming to {} ({})", to, err));
                    log::error!("{}", error);
                    Err(error)
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
            let fixture = testlib::TestFixture::create();
            let filename = "test_file.dat";
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
            // set up the test case
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