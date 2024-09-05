//! The weather data administration API.
use super::{backend, Result};
use std::path::PathBuf;

/// Create an instance of the weather data administration `API`.
///
/// # Arguments
///
/// * `dirname` is the weather data directory pathname.
pub fn create_weather_admin(dirname: &str) -> Result<WeatherAdmin> {
    WeatherAdmin::new(dirname)
}
pub fn weather_admin(dirname: Option<PathBuf>) -> Result<WeatherAdmin> {
    let dirname = dirname.map_or(Default::default(), |pathbuf| pathbuf.as_path().display().to_string());
    WeatherAdmin::new(dirname.as_str())
}

/// Give library users access to the administration API.
pub use api::WeatherAdmin;
mod api {
    //! The administration commands are scoped to this module.
    use super::{admin_entities::UsCitiesInfo, *};
    use crate::entities::DataCriteria;
    use admin_entities::{Components, DbMode};
    use backend::{
        db::admin as db_admin,
        filesys::{admin as fs_admin, weather_dir, MigrateConfig, WeatherDir},
    };
    use std::path::PathBuf;
    use toolslib::{fmt::commafy, stopwatch::StopWatch};

    /// The weather data administration `API`.
    #[derive(Debug)]
    pub struct WeatherAdmin(
        /// The weather data directory.
        WeatherDir,
    );
    impl WeatherAdmin {
        /// Create an instance of the weather data administration `API`.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the weather data directory pathname.
        pub(super) fn new(dirname: &str) -> Result<Self> {
            Ok(WeatherAdmin(weather_dir(dirname)?))
        }
        /// Initialize the weather database using the supplied database configuration.
        ///
        /// # Arguments
        ///
        /// * `db_mode` is the database configuration to initialize.
        /// * `drop` when `true` will delete the schema before initialization.
        /// * `load` when `true` will load weather data into the database.
        pub fn init(&self, db_mode: DbMode, drop: bool, load: bool, threads: usize) -> Result<()> {
            db_admin::init_db(&self.0, db_mode, drop, load, threads)?;
            Ok(())
        }
        /// Deletes the weather database schema and optionally deletes the database.
        ///
        /// # Arguments
        ///
        /// * `delete` when `true` will delete the database file.
        pub fn drop(&self, delete: bool) -> Result<()> {
            db_admin::drop_db(&self.0, delete)?;
            Ok(())
        }
        /// Provides information about the weather data archives and database.
        pub fn components(&self) -> Result<Components> {
            let fs_details = fs_admin::filesys_details(&self.0)?;
            let db_details = db_admin::db_details(&self.0)?;
            Ok(Components { db_details, fs_details })
        }
        /// Convert *DarkSky* archives into [History](crate::entities::History) archives.
        ///
        /// # Arguments
        ///
        /// * `into` identifies the directory where converted archive will be written.
        /// * `create` indicates the directory should be created if it does not exist.
        /// * `retain` indicates existing converted archives should not be deleted before adding documents.
        /// * `criteria` identifies what location archives should be converted.
        pub fn migrate(&self, into: PathBuf, create: bool, retain: bool, criteria: DataCriteria) -> Result<usize> {
            let count = fs_admin::migrate_history(MigrateConfig { source: &self.0, create, retain, criteria }, into)?;
            Ok(count)
        }
        /// Reload history for locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations that will be reloaded.
        pub fn reload(&self, criteria: DataCriteria) -> Result<usize> {
            let locations = db_admin::reload(&self.0, criteria)?;
            Ok(locations.len())
        }
        /// Load the US Cities database.
        ///
        /// # Arguments
        ///
        /// * `uscities_path` is the US Cities `CSV` file that will populate the database.
        pub fn uscities_load(&self, uscities_path: &PathBuf) -> Result<()> {
            let stopwatch = StopWatch::start_new();
            let count = db_admin::uscities_load(&self.0, uscities_path)?;
            log::debug!("Loaded {} US Cities in {}", commafy(count), stopwatch);
            Ok(())
        }
        /// Delete the US Cities database.
        pub fn uscities_delete(&self) -> Result<()> {
            db_admin::uscities_delete(&self.0)?;
            Ok(())
        }
        /// Show information about the US Cities database.
        pub fn uncities_info(&self) -> Result<UsCitiesInfo> {
            let cities_info = db_admin::uscities_info(&self.0)?;
            Ok(cities_info)
        }
    }
}

pub(crate) mod admin_entities {
    //! The data model for weather data administration.
    #[derive(Clone, Debug, PartialEq)]
    pub enum DbMode {
        Hybrid,
        Document(bool),
        Normalized,
    }
    impl std::fmt::Display for DbMode {
        /// Create a description of the database mode.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DbMode::Hybrid => write!(f, "Hybrid DB"),
                DbMode::Document(compressed) => match compressed {
                    true => write!(f, "Document DB (compressed"),
                    false => write!(f, "Document DB"),
                },
                DbMode::Normalized => write!(f, "Normalized DB"),
            }
        }
    }

    /// The administration `stat` information.
    #[derive(Debug)]
    pub struct Components {
        /// The database information.
        pub db_details: Option<DbDetails>,
        /// The archive information.
        pub fs_details: FilesysDetails,
    }

    /// The database information.
    #[derive(Debug)]
    pub struct DbDetails {
        /// The database configuration.
        pub mode: DbMode,
        /// The size of the database.
        pub size: usize,
        /// The location weather history information.
        pub location_details: Vec<LocationDetails>,
    }

    /// Information about the weather history archives.
    #[derive(Debug, Default)]
    pub struct FilesysDetails {
        /// The total size of weather history archives.
        pub size: usize,
        /// The location information
        pub location_details: Vec<LocationDetails>,
    }

    /// Weather history metadata for a [location](crate::prelude::Location).
    #[derive(Debug)]
    pub struct LocationDetails {
        /// The location alias name.
        pub alias: String,
        /// The number of bytes being used to hold weather history information.
        pub size: usize,
        /// The count of weather histories the [location](crate::prelude::Location) has available.
        pub histories: usize,
    }

    #[derive(Debug)]
    pub struct UsCitiesInfo {
        pub db_size: usize,
        pub state_info: Vec<(String, usize)>,
    }
}
