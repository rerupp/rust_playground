//! Isolates the administration API from the weather API.
use super::{weather_locations, ArchiveMd, Error, Result, WeatherArchive, WeatherDir, WeatherFile};
pub(crate) use v2::{filesys_details, migrate_history, MigrateConfig};
mod v2 {
    //! The current implementation of administration for the file system.
    use super::*;
    use crate::{
        admin::entities::{FilesysDetails, LocationDetails},
        entities::{DataCriteria, History, Location},
    };
    use chrono::{DateTime, NaiveDate};
    use std::{
        env, fs,
        io::Read,
        path::{Path, PathBuf},
    };
    use zip::read::ZipFile;

    /// The common error builder
    macro_rules! error {
        ($reason:expr) => {
            Err(Error::from($reason))
        };
    }

    pub fn filesys_details(weather_dir: &WeatherDir) -> Result<FilesysDetails> {
        let locations = weather_locations(weather_dir)?;
        let mut location_details = Vec::with_capacity(locations.cnt());
        let mut archives_size: u64 = 0;
        for location in locations.as_iter(&vec![], false, false) {
            let file = weather_dir.archive(&location.alias);
            archives_size += file.size();
            let mut histories: usize = 0;
            let mut history_dates = vec![];
            let size = WeatherArchive::open(&location.alias, file)?
                .iter_date_range(None, false, ArchiveMd::new)?
                .map(|md| {
                    histories += 1;
                    history_dates.push(md.date);
                    md.compressed_size as usize
                })
                .sum::<usize>();
            location_details.push(LocationDetails { alias: location.alias.clone(), size, histories })
        }
        Ok(FilesysDetails { size: archives_size as usize, location_details })
    }

    #[derive(Debug)]
    /// The metadata surrounding migrating old data to [History].
    pub struct MigrateConfig<'w> {
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
    pub fn migrate_history(config: MigrateConfig, target: PathBuf) -> Result<usize> {
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
        pub fn migrate(
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
            let migrations: Vec<MigrationData> = source.iter_date_range(None, false, MigrationData::new)?.collect();
            let mut histories: Vec<History> = Vec::with_capacity(migrations.len());
            for md in migrations {
                let darksky = md.to_darksky()?;
                histories.push(darksky.as_history(alias, &md.date));
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
            fn as_history(&self, alias: &str, date: &NaiveDate) -> History {
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
                    sunrise: daily
                        .sunriseTime
                        .map_or(None, |ts| DateTime::from_timestamp(ts, 0).map_or(None, |dt| Some(dt.naive_utc()))),
                    sunset: daily
                        .sunsetTime
                        .map_or(None, |ts| DateTime::from_timestamp(ts, 0).map_or(None, |dt| Some(dt.naive_utc()))),
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
                        let probabilities: Vec<f64> = hourly_iter!(self).filter_map(|h| h.precipProbability).collect();
                        if probabilities.is_empty() {
                            log::trace!("probabilities has no value, using default.");
                            None
                        } else {
                            let chance = probabilities.iter().sum::<f64>() / probabilities.len() as f64;
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
                    Some(p) => p * 24f64,
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
        /// The *DarkSky* daily weather history data.
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
