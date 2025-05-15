//! The weather data administration API.

use super::Result;
use std::path::PathBuf;

/// Create an instance of the weather data administration `API`.
///
/// # Arguments
///
/// * `dirname` is the weather data directory pathname.
pub fn weather_admin(dirname: Option<PathBuf>) -> Result<WeatherAdmin> {
    let dirname = dirname.map_or(Default::default(), |pb| pb.as_path().display().to_string());
    WeatherAdmin::new(dirname.as_str())
}

/// Give library users access to the administration API.
pub use api::WeatherAdmin;

mod api {
    //! The administration commands are scoped to this module.
    use super::{PathBuf, Result};
    use crate::{
        admin::entities::{Components, UsCitiesInfo},
        backend::{
            db,
            filesys::{self as fs, weather_dir, WeatherDir},
        },
        entities::DataCriteria,
    };
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
        pub fn new(dirname: &str) -> Result<Self> {
            Ok(WeatherAdmin(weather_dir(dirname)?))
        }
        /// Initialize the weather database using the supplied database configuration.
        ///
        /// # Arguments
        ///
        /// * `drop` when `true` will delete the schema before initialization.
        /// * `load` when `true` will load weather data into the database.
        pub fn init(&self, drop: bool, load: bool, threads: usize) -> Result<()> {
            db::admin::init_db(&self.0, drop, load, threads)?;
            Ok(())
        }
        /// Deletes the weather database schema and optionally deletes the database.
        ///
        /// # Arguments
        ///
        /// * `delete` when `true` will delete the database file.
        pub fn drop(&self, delete: bool) -> Result<()> {
            db::admin::drop_db(&self.0, delete)?;
            Ok(())
        }
        /// Provides information about the weather data archives and database.
        pub fn components(&self) -> Result<Components> {
            let fs_details = fs::admin::filesys_details(&self.0)?;
            let db_details = db::admin::db_details(&self.0)?;
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
            let migrate_config = fs::admin::MigrateConfig { source: &self.0, create, retain, criteria };
            let count = fs::admin::migrate_history(migrate_config, into)?;
            Ok(count)
        }
        /// Reload history for locations.
        ///
        /// # Arguments
        ///
        /// * `criteria` identifies the locations that will be reloaded.
        pub fn reload(&self, criteria: DataCriteria) -> Result<usize> {
            let locations = db::admin::reload(&self.0, criteria)?;
            Ok(locations.len())
        }
        /// Load the US Cities database.
        ///
        /// # Arguments
        ///
        /// * `uscities_path` is the US Cities `CSV` file that will populate the database.
        pub fn uscities_load(&self, uscities_path: &PathBuf) -> Result<()> {
            let stopwatch = StopWatch::start_new();
            let count = db::admin::uscities_load(&self.0, uscities_path)?;
            log::debug!("Loaded {} US Cities in {}", commafy(count), stopwatch);
            Ok(())
        }
        /// Delete the US Cities database.
        pub fn uscities_delete(&self) -> Result<()> {
            db::admin::uscities_delete(&self.0)?;
            Ok(())
        }
        /// Show information about the US Cities database.
        pub fn uscities_info(&self) -> Result<UsCitiesInfo> {
            let cities_info = db::admin::uscities_info(&self.0)?;
            Ok(cities_info)
        }
    }
}

pub(crate) mod entities {
    //! The data model for weather data administration.

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
        // /// The database configuration.
        // pub mode: DbMode,
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