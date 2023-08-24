//! The implementations of weather data.

pub(crate) mod db;
pub(crate) mod filesys;

use crate::entities::{DailyHistories, DataCriteria, DateRange, HistoryDates, HistorySummaries, Location};
use std::{fmt, result};

/// The result of calling an API in the backend.
pub type Result<T> = result::Result<T, Error>;

/// The error that can be returned by the backend.
#[derive(Debug)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// Create an API error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(error)
    }
}
/// Create an API error from a string reference.
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(error.to_string())
    }
}

/// The `API` common to all the backend implementations.
pub(crate) trait DataAdapter {
    /// Returns the daily weather data history for a location.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies what location should be used.
    /// * `history_range` specifies the date range that should be used.
    fn daily_histories(&self, criteria: DataCriteria, history_range: DateRange) -> Result<DailyHistories>;
    /// Get the weather history dates for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations.
    fn history_dates(&self, criteria: DataCriteria) -> Result<Vec<HistoryDates>>;
    /// Get a summary of the weather history available for locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations that should be used.
    fn history_summaries(&self, criteria: DataCriteria) -> Result<Vec<HistorySummaries>>;
    /// Get the metadata for weather locations.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies the locations of interest.
    fn locations(&self, criteria: DataCriteria) -> Result<Vec<Location>>;
}

#[cfg(test)]
pub(in crate::backend) mod testlib {
    //! A library for common utilities used by the backend.

    use rand::Rng;
    use std::{env, fmt, fs, path};

    /// Used to create a temporary weather directory and delete it as part of the function exit.
    #[derive(Debug)]
    pub(in crate::backend) struct TestFixture(path::PathBuf);
    impl TestFixture {
        /// Creates a test weather directory or panics if a unique directory cannot be created.
        pub(in crate::backend) fn create() -> Self {
            let tmpdir = env::temp_dir();
            let mut weather_dir: Option<path::PathBuf> = None;
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
                    Ok(_) => Self(root_dir),
                    Err(e) => {
                        panic!("Error creating '{}': {}", root_dir.as_path().display(), e.to_string())
                    }
                },
                None => panic!("Tried 10 times to get a unique test directory name and failed..."),
            }
        }
        pub(in crate::backend) fn copy_resources(&self, source: &path::PathBuf) {
            if source.is_file() {
                if let Err(err) = fs::copy(source, &self.0) {
                    panic!("Error copying {} to {} ({}).", source.as_path().display(), self, &err);
                }
            } else {
                let paths = fs::read_dir(&source).unwrap();
                for entry in paths {
                    let source_path = entry.unwrap().path();
                    let target_path = self.0.join(source_path.file_name().unwrap().to_str().unwrap());
                    println!("target {}", target_path.as_path().display());
                    if let Err(err) = fs::copy(&source_path, &target_path) {
                        panic!("Error copying {} to {} ({}).", source_path.as_path().display(), self, &err);
                    }
                }
            }
        }
    }
    impl Drop for TestFixture {
        /// Clean up the temporary directory as best you can.
        fn drop(&mut self) {
            if let Err(e) = fs::remove_dir_all(self.to_string()) {
                eprintln!("Yikes... Error cleaning up test weather_dir: {}", e.to_string());
            }
        }
    }
    impl fmt::Display for TestFixture {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0.as_path().display())
        }
    }
    impl From<&TestFixture> for path::PathBuf {
        fn from(value: &TestFixture) -> Self {
            path::PathBuf::from(value.to_string())
        }
    }

    pub(in crate::backend) fn generate_random_string(len: usize) -> String {
        let mut rand = rand::thread_rng();
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmonopqrstuvwxyz0123456789";
        let random_string = (0..len)
            .map(|_| {
                let idx = rand.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect();
        // eprintln!("generate_random_string: {}...", random_string);
        random_string
    }

    pub(in crate::backend) fn test_resources() -> path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("tests")
    }
}
