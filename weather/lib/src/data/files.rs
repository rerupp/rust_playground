//! # The DataAPI filesystem implementation.
//!
//! This module implements the API used to access weather data. It uses a local filesystem
//! to persist weather data. The weather data consists of `JSON` documents and `ZIP archives`
//! for each location.
//!
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use chrono::prelude::*;
use serde_json::Value;

use super::domain::{DailyHistories, DailyHistory, HistoryDates, HistoryRange, HistorySummary, Location, Locations};

use super::{DailyHistoryQuery, DataAPI, Error, HistoryQuery, LocationQuery, Result};

/// The contents of the filesystem `DataAPI` implementation.
#[derive(Debug)]
pub struct FsData {
    /// The path where weather data is stored.
    data_dir: PathBuf,
}

/// The filesystem implementation of weather data.
impl FsData {
    /// Create an instance of the filesystem based DataAPI.
    ///
    /// If the pathname is empty, a check is made to see if the environment variable
    /// `**WEATHER_DATA**` exists. If the variable is found it will be used as the
    /// pathname. If the variable is not found the directory named `weather_data` will
    /// be used.
    ///
    /// An error is returned if the pathname does not exist or is not a directory.
    ///
    /// * `preferred_pathname` - the directory to use or an empty string.
    ///
    pub(crate) fn new(preferred_pathname: &str) -> Result<Box<dyn DataAPI>> {
        let root_pathname = match preferred_pathname.is_empty() {
            false => String::from(preferred_pathname),
            true => {
                if let Ok(env_pathname) = std::env::var("WEATHER_DATA") {
                    env_pathname.clone()
                } else {
                    String::from("weather_data")
                }
            }
        };

        let data_dir = PathBuf::from(&root_pathname);
        log::trace!("data dir: {}", data_dir.as_path().display());
        if !data_dir.is_dir() {
            let error_msg = format!("{} is not a directory!", root_pathname);
            log::error!("{error_msg}");
            Err(Error::from(error_msg))
        } else {
            Ok(Box::new(FsData { data_dir }))
        }
    }
}

/// The filesystem implementation of the `DataAPI`.
impl DataAPI for FsData {
    /// Returns daily history for a location.
    ///
    /// An [archive reader](archive::Reader) is used to mine location weather data.
    ///
    /// # Arguments
    ///
    /// * `query` - identifies the location and dates of the returned weather data.
    ///
    fn get_daily_history(&self, query: DailyHistoryQuery) -> Result<DailyHistories> {
        let mut reader = archive::Reader::new(&query.location_id, &self.data_dir)?;
        let mut daily_histories: Vec<DailyHistory> = vec![];
        for i in 0..reader.file_count() {
            let zip_file = reader.get(i)?;
            let name = zip_file.name().to_string();
            let file_date = archive::date_from_name(&name)?;
            if query.history_bounds.contains(&file_date) {
                let root = value_from_reader(zip_file)?;
                if let Some(daily_history) = make_daily_history(&root, &query.location_id, file_date) {
                    daily_histories.push(daily_history);
                } else {
                    log::error!("{}", format!("Error getting history for {}: {}...", &query.location_id, name));
                }
            }
        }
        Ok(DailyHistories { location_id: query.location_id, daily_histories })
    }

    /// Returns a summary of the weather data available to a location.
    ///
    /// An [archive reader](archive::Reader) is used to mine location weather data.
    ///
    /// # Arguments
    ///
    /// * `history_query` - identifies the location and how the summary data should be returned.
    ///
    fn get_history_summary(&self, history_query: HistoryQuery) -> Result<HistorySummary> {
        let location_id = history_query.location_id.as_str();
        let mut reader = archive::Reader::new(location_id, &self.data_dir)?;
        let history_count = reader.file_count();
        let mut raw_size: usize = 0;
        let mut compressed_size: usize = 0;
        for i in 0..history_count {
            let zip_file = reader.get(i)?;
            raw_size += zip_file.size() as usize;
            compressed_size += zip_file.compressed_size() as usize;
        }
        Ok(HistorySummary {
            location_id: history_query.location_id,
            count: history_count,
            raw_size: Some(raw_size),
            overall_size: Some(reader.archive_size()),
            compressed_size: Some(compressed_size),
        })
    }

    /// Returns the weather data dates available to a location.
    ///
    /// An [archive reader](archive::Reader) is used to mine location weather data.
    ///
    /// # Arguments
    ///
    /// * `query` - identifies the location and how the summary data should be returned.
    ///
    fn get_history_dates(&self, query: HistoryQuery) -> Result<HistoryDates> {
        let location_id = query.location_id.as_str();
        let mut reader = archive::Reader::new(location_id, &self.data_dir)?;
        let history_count = reader.file_count();
        let mut dates: Vec<NaiveDate> = vec![];
        for i in 0..history_count {
            let zip_file = reader.get(i)?;
            match archive::date_from_name(&zip_file.name()) {
                Ok(d) => dates.push(d),
                Err(e) => log::error!("{}", e.to_string()),
            }
        }
        Ok(HistoryDates { location_id: location_id.to_string(), history_ranges: HistoryRange::from_dates(&dates) })
    }

    /// Returns the properties of locations.
    ///
    /// An [archive reader](archive::Reader) is used to mine location weather data.
    ///
    /// # Arguments
    ///
    /// * `location_query` - identifies the locations and how data should be returned.
    ///
    fn get_location_data(&self, location_query: LocationQuery) -> Result<Locations> {
        let include_location = |name: &String, alias: &String| -> bool {
            if location_query.filters.is_empty() {
                true
            } else {
                let name_pat = if location_query.case_insensitive { name.to_lowercase() } else { name.clone() };
                let alias_pat = if location_query.case_insensitive { alias.to_lowercase() } else { alias.clone() };
                location_query.filters.iter().find(|&p| name_pat.starts_with(p) || alias_pat.starts_with(p)).is_some()
            }
        };

        let locations_path = self.data_dir.join("locations.json");
        let root = value_from_path(locations_path)?;
        if !root.is_object() {
            Err(Error::from("The locations root node is not an object..."))
        } else {
            let locations_value = root.get("locations");
            if locations_value.is_none() {
                Err(Error::from("Did not find the array of locations..."))
            } else {
                let mut locations: Locations = vec![];
                if let Value::Array(locations_array) = locations_value.unwrap() {
                    for location_value in locations_array {
                        let name = value_as_string(location_value.get("name"));
                        let alias = value_as_string(location_value.get("alias"));
                        if include_location(&name, &alias) {
                            locations.push(Location {
                                id: alias.to_lowercase(),
                                name,
                                alias,
                                longitude: value_as_string(location_value.get("longitude")),
                                latitude: value_as_string(location_value.get("latitude")),
                                tz: value_as_string(location_value.get("tz")),
                            });
                        }
                    }
                }
                if location_query.sort {
                    locations.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
                }
                Ok(locations)
            }
        }
    }
}

/// Returns daily history mined from the JSON document.
///
/// `None` will be returned if the daily history node cannot be found in the document.
///
/// # Arguments
///
/// * `root` - the head of the JSON document.
/// * `location_id` - the id of the location.
/// * `date` - the date associated with the weather data.
///
pub fn make_daily_history(root: &Value, location_id: &str, date: NaiveDate) -> Option<DailyHistory> {
    let daily = &root["daily"]["data"][0];
    if daily.is_object() {
        Some(DailyHistory {
            location_id: location_id.to_string(),
            date,
            temperature_high: f64_from_value(daily.get("temperatureHigh")),
            temperature_high_time: i64_from_value(daily.get("temperatureHighTime")),
            temperature_low: f64_from_value(daily.get("temperatureLow")),
            temperature_low_time: i64_from_value(daily.get("temperatureLowTime")),
            temperature_max: f64_from_value(daily.get("temperatureMax")),
            temperature_max_time: i64_from_value(daily.get("temperatureMaxTime")),
            temperature_min: f64_from_value(daily.get("temperatureMin")),
            temperature_min_time: i64_from_value(daily.get("temperatureMinTime")),
            wind_speed: f64_from_value(daily.get("windSpeed")),
            wind_gust: f64_from_value(daily.get("windGust")),
            wind_gust_time: i64_from_value(daily.get("windGustTime")),
            wind_bearing: i64_from_value(daily.get("windBearing")),
            cloud_cover: f64_from_value(daily.get("cloudCover")),
            uv_index: i64_from_value(daily.get("uvIndex")),
            uv_index_time: i64_from_value(daily.get("uvIndexTime")),
            summary: string_from_value(daily.get("summary")),
            humidity: f64_from_value(daily.get("humidity")),
            dew_point: f64_from_value(daily.get("dewPoint")),
            sunrise_time: i64_from_value(daily.get("sunriseTime")),
            sunset_time: i64_from_value(daily.get("sunsetTime")),
            moon_phase: f64_from_value(daily.get("moonPhase")),
        })
    } else {
        log::warn!("JSON root is not an object!!!");
        None
    }
}

/// Returns a **JSON** document read from the path argument.
///
/// An error will be returned if the file cannot be opened or if the file does not
/// contain a valid **JSON** document.
///
/// # Arguments
///
/// * `json_path` - The **JSON** file pathname.
///
fn value_from_path(json_path: PathBuf) -> Result<Value> {
    let json_file = File::open(&json_path)?;
    let json_reader = BufReader::new(json_file);
    value_from_reader(json_reader)
}

/// Return a **JSON** document.
///
/// An error will be returned if the data read is not a valid **JSON** document.
///
/// # Arguments
///
/// * `reader` - the reader that will be used to create the **JSON** document.
///
fn value_from_reader<R: std::io::Read>(reader: R) -> Result<Value> {
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

/// Returns an integer if the value is not `None`.
///
/// # Arguments
///
/// * `value` - the value that will be converted to an integer.
///
#[inline]
pub fn i64_from_value(value: Option<&Value>) -> Option<i64> {
    value.map_or(None, |v| v.as_i64())
}

/// Returns a float if the value is not `None`.
///
/// # Arguments
///
/// * `value` - the value that will be converted to a float.
///
#[inline]
pub fn f64_from_value(value: Option<&Value>) -> Option<f64> {
    value.map_or(None, |v| v.as_f64())
}

/// Returns a string if the value is not `None`.
///
/// # Arguments
///
/// * `value` - the value that will be converted to a string.
///
pub fn string_from_value(value: Option<&Value>) -> Option<String> {
    if let Some(value) = value {
        Some(match value {
            Value::Number(n) => n.to_string(),
            _ => value.as_str().map_or(String::default(), |s| s.to_string()),
        })
    } else {
        None
    }
}

/// Returns a string if the value is not `None`, an empty string otherwise.
///
/// # Arguments
///
/// * `value` - the value that will be converted to a string.
///
#[inline]
pub fn value_as_string(value: Option<&Value>) -> String {
    string_from_value(value).unwrap_or("".to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use toolslib::date_time::{get_date, get_time};
    use super::*;

    #[test]
    fn from_value_all() {
        let mk_timestamp = |hour: u32| -> i64 {
            NaiveDateTime::new(get_date(2022, 7, 15), get_time(hour, 0, 0)).timestamp()
        };
        let temperature_high: f64 = 70.0;
        let temperature_high_time = mk_timestamp(1);
        let temperature_low: f64 = 69.9;
        let temperature_low_time = mk_timestamp(2);
        let temperature_max: f64 = 70.1;
        let temperature_max_time = mk_timestamp(3);
        let temperature_min: f64 = 69.8;
        let temperature_min_time = mk_timestamp(4);
        let wind_speed: f64 = 10.0;
        let wind_gust: f64 = 15.1;
        let wind_gust_time = mk_timestamp(5);
        let wind_bearing: i64 = 90;
        let cloud_cover: f64 = 0.3;
        let uv_index: i64 = 10;
        let uv_index_time = mk_timestamp(6);
        let summary: String = "a summary".to_string();
        let humidity: f64 = 30.5;
        let dew_point: f64 = 40.5;
        let sunrise_time = mk_timestamp(7);
        let sunset_time = mk_timestamp(8);
        let moon_phase: f64 = 0.5;
        let value = json!({
            "daily": {
                "data": [
                    {
                        "temperatureHigh": temperature_high,
                        "temperatureHighTime": temperature_high_time,
                        "temperatureLow": temperature_low,
                        "temperatureLowTime": temperature_low_time,
                        "temperatureMax": temperature_max,
                        "temperatureMaxTime": temperature_max_time,
                        "temperatureMin": temperature_min,
                        "temperatureMinTime": temperature_min_time,
                        "windSpeed": wind_speed,
                        "windGust": wind_gust,
                        "windGustTime": wind_gust_time,
                        "windBearing": wind_bearing,
                        "cloudCover": cloud_cover,
                        "uvIndex": uv_index,
                        "uvIndexTime": uv_index_time,
                        "summary": summary,
                        "humidity": humidity,
                        "dewPoint": dew_point,
                        "sunriseTime": sunrise_time,
                        "sunsetTime": sunset_time,
                        "moonPhase": moon_phase,
                    }
                ]
            }
        });
        let location_id = "an_id";
        let date = NaiveDate::from_ymd_opt(2022, 7, 14).unwrap();
        let daily_history = make_daily_history(&value, location_id, date.clone()).unwrap();
        assert_eq!(daily_history.location_id, location_id.to_string());
        assert_eq!(daily_history.date, date);
        assert_eq!(daily_history.temperature_high, Some(temperature_high));
        assert_eq!(daily_history.temperature_high_time, Some(temperature_high_time));
        assert_eq!(daily_history.temperature_low, Some(temperature_low));
        assert_eq!(daily_history.temperature_low_time, Some(temperature_low_time));
        assert_eq!(daily_history.temperature_max, Some(temperature_max));
        assert_eq!(daily_history.temperature_max_time, Some(temperature_max_time));
        assert_eq!(daily_history.temperature_min, Some(temperature_min));
        assert_eq!(daily_history.temperature_min_time, Some(temperature_min_time));
        assert_eq!(daily_history.wind_speed, Some(wind_speed));
        assert_eq!(daily_history.wind_gust, Some(wind_gust));
        assert_eq!(daily_history.wind_gust_time, Some(wind_gust_time));
        assert_eq!(daily_history.wind_bearing, Some(wind_bearing));
        assert_eq!(daily_history.cloud_cover, Some(cloud_cover));
        assert_eq!(daily_history.uv_index, Some(uv_index));
        assert_eq!(daily_history.uv_index_time, Some(uv_index_time));
        assert_eq!(daily_history.summary, Some(summary));
        assert_eq!(daily_history.humidity, Some(humidity));
        assert_eq!(daily_history.dew_point, Some(dew_point));
        assert_eq!(daily_history.sunrise_time, Some(sunrise_time));
        assert_eq!(daily_history.sunset_time, Some(sunset_time));
        assert_eq!(daily_history.moon_phase, Some(moon_phase));
    }

    #[test]
    fn from_value_none() {
        let value = json!({
            "daily": {
                "data": [
                    {
                    }
                ]
            }
        });
        let location_id = "some_id";
        let date = NaiveDate::from_ymd_opt(2022, 7, 15).unwrap();
        let daily_history = make_daily_history(&value, location_id, date.clone()).unwrap();
        assert_eq!(daily_history.location_id, location_id.to_string());
        assert_eq!(daily_history.date, date);
        assert_eq!(daily_history.temperature_high, None);
        assert_eq!(daily_history.temperature_high_time, None);
        assert_eq!(daily_history.temperature_low, None);
        assert_eq!(daily_history.temperature_low_time, None);
        assert_eq!(daily_history.temperature_max, None);
        assert_eq!(daily_history.temperature_max_time, None);
        assert_eq!(daily_history.temperature_min, None);
        assert_eq!(daily_history.temperature_min_time, None);
        assert_eq!(daily_history.wind_speed, None);
        assert_eq!(daily_history.wind_gust, None);
        assert_eq!(daily_history.wind_gust_time, None);
        assert_eq!(daily_history.wind_bearing, None);
        assert_eq!(daily_history.cloud_cover, None);
        assert_eq!(daily_history.uv_index, None);
        assert_eq!(daily_history.uv_index_time, None);
        assert_eq!(daily_history.summary, None);
        assert_eq!(daily_history.humidity, None);
        assert_eq!(daily_history.dew_point, None);
        assert_eq!(daily_history.sunrise_time, None);
        assert_eq!(daily_history.sunset_time, None);
        assert_eq!(daily_history.moon_phase, None);
    }

    #[test]
    fn number_converters() {
        assert_eq!(i64_from_value(None), None);
        assert_eq!(f64_from_value(None), None);
        let number = json!(123);
        assert_eq!(i64_from_value(Some(&number)), Some(123));
        assert_eq!(f64_from_value(Some(&number)), Some(123.0));
        let float_number = json!(456.0);
        assert_eq!(i64_from_value(Some(&float_number)), None);
        assert_eq!(f64_from_value(Some(&float_number)), Some(456.0));
        let bad_number = json!("789");
        assert_eq!(i64_from_value(Some(&bad_number)), None);
        assert_eq!(f64_from_value(Some(&bad_number)), None);
    }

    #[test]
    fn string_converters() {
        let string = json!("123");
        assert_eq!(string_from_value(Some(&string)), Some("123".to_string()));
        assert_eq!(value_as_string(Some(&string)), "123".to_string());
        let integer = json!(123);
        assert_eq!(string_from_value(Some(&integer)), Some(123.to_string()));
        assert_eq!(value_as_string(Some(&integer)), 123.to_string());
    }

    #[test]
    pub fn good_path() {
        assert!(FsData::new(".").is_ok());
    }

    #[test]
    pub fn bad_path() {
        assert!(FsData::new("./something_that_should_not_exist").is_err());
    }
}

/// The code providing access into a **ZIP** archive.
///
/// This module uses the `zip` dependency.
///
mod archive {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    use zip::read::ZipFile;
    use zip::{ZipArchive, ZipWriter};

    use super::*;

    /// A **ZIP** archive reader.
    #[derive(Debug)]
    pub(crate) struct Reader {
        /// A `Path` to the archive location.
        archive_path: PathBuf,
        /// The **ZIP** file archive to read.
        zip_archive: ZipArchive<BufReader<File>>,
    }

    /// Create an empty ZIP archive
    /// 
    /// An error will be returned if there are problems trying to creaate the empy archive.
    /// 
    /// # Arguments
    /// 
    /// * `archive_path` is the pathname of the new emtpy archive.
    fn create_archive(archive_path: &PathBuf) -> Result<()> {
        match File::options().create(true).write(true).open(&archive_path) {
            Ok(writer) => {
                let mut zip_archive = ZipWriter::new(writer);
                match zip_archive.finish() {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        log::error!("Could not initialize archive {}: {}", archive_path.as_path().display(), err);
                        Err(Error::from(err))
                    }
                }
            }
            Err(err) => {
                log::error!("Could not create {}: {}", archive_path.as_path().display(), err);
                Err(Error::from(err))
            }
        }
    }

    impl Reader {
        /// Creates a new instance of the reader.
        ///
        /// An error will be returned if the file cannot be opened or the file is not a **ZIP**
        /// archive.
        ///
        /// # Arguments
        ///
        /// * `location_id` - the location identifier.
        /// * `parent_dir` - a path to the directory containing the archive.
        ///
        pub(crate) fn new(location_id: &str, parent_dir: &PathBuf) -> Result<Reader> {
            let archive_path = parent_dir.join(location_id).with_extension("zip");
            if !archive_path.exists() {
                log::warn!("{} does not exist, creating one.", archive_path.as_path().display());
                create_archive(&archive_path)?;
            }
            let archive_file = match File::open(&archive_path) {
                Ok(file) => Ok(file),
                Err(error) => {
                    log::error!("Error opening {}: {error}", archive_path.as_path().display());
                    Err(Error::from(error))
                }
            }?;
            let archive_reader = BufReader::new(archive_file);
            let zip_archive = match ZipArchive::new(archive_reader) {
                Ok(zip_archive) => Ok(zip_archive),
                Err(error) => {
                    log::error!("Error reading {}: {error}", archive_path.as_path().display());
                    Err(Error::from(error))
                }
            }?;
            Ok(Reader { archive_path, zip_archive })
        }

        /// Returns the filesystem size of the archive.
        ///
        pub(crate) fn archive_size(&self) -> usize {
            self.archive_path.metadata().map_or(0, |m| m.len() as usize)
        }

        /// Returns the number of files in the **ZIP** archive.
        ///
        pub(crate) fn file_count(&self) -> usize {
            self.zip_archive.len()
        }

        /// Returns a reader that can retrieve information for a file in an archive.
        ///
        /// # Arguments
        ///
        /// * `file_number` - the file index within the **ZIP** archive.
        ///
        pub(crate) fn get(&mut self, file_number: usize) -> Result<ZipFile> {
            match self.zip_archive.by_index(file_number) {
                Ok(zip_file) => Ok(zip_file),
                Err(error) => {
                    log::error!("Error getting file from {}: {error}", self.archive_path.as_path().display());
                    Err(Error::from(error))
                }
            }
        }
    }

    /// Returns a weather data date encoded into the filename.
    ///
    /// An error is returned if the filename does not end in `.json` or if the
    /// ISO8601 date within the filename is not found.
    ///
    /// # Arguments
    ///
    /// * `name` - the **ZIP** archive internal filename containing the embedded date.
    ///
    pub(crate) fn date_from_name(name: &str) -> Result<NaiveDate> {
        if !name.ends_with(".json") {
            Err(Error::from("Expected name to end in '.json'..."))
        } else {
            let ymd_offset = "yyyymmdd.json".len();
            if name.len() < ymd_offset {
                Err(Error::from("The format of the name is incorrect..."))
            } else {
                let ymd_index = name.len() - ymd_offset;
                let ymd: &str = &name[ymd_index..ymd_index + 8];
                if !ymd.chars().all(char::is_numeric) {
                    Err(Error::from("The name date '{ymd}' was not all digits..."))
                } else {
                    let year = ymd[..4].parse().unwrap();
                    let month = ymd[4..6].parse().unwrap();
                    let day = ymd[6..].parse().unwrap();
                    match NaiveDate::from_ymd_opt(year, month, day) {
                        Some(date) => Ok(date),
                        None => Err(Error::from(format!("to_date: '{}' month or day out of bounds...", ymd))),
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        // use serde_json::{to_string_pretty, Value};

        use super::*;

        #[test]
        pub fn parse_bad_name() {
            assert!(date_from_name("00000000.jso").is_err());
            assert!(date_from_name("0000000.json").is_err());
            assert!(date_from_name("yyyy0000.json").is_err());
            assert!(date_from_name("0000mm00.json").is_err());
            assert!(date_from_name("000000dd.json").is_err());
            assert!(date_from_name("20220732.json").is_err());
            assert!(date_from_name("20221731.json").is_err());
        }

        #[test]
        pub fn parse_date_from_name() {
            assert_eq!(
                NaiveDate::from_ymd_opt(2020, 4, 29).unwrap(),
                date_from_name("mesa/mesa-20200429.json").unwrap()
            );
            assert_eq!(NaiveDate::from_ymd_opt(2022, 10, 31).unwrap(), date_from_name("020221031.json").unwrap());
        }

        // #[test]
        // pub fn get_zipfile_content() {
        //     let data_dir = PathBuf::from("weather_data");
        //     let mut reader = Reader::new("tigard_or", &data_dir).unwrap();
        //     let zip_file = reader.get(0).unwrap();
        //     let reader = BufReader::new(zip_file);
        //     let json: Value = serde_json::from_reader(reader).unwrap();
        //     println!("{}", &to_string_pretty(&json).expect("Error printing json..."));
        // }
    }
}
