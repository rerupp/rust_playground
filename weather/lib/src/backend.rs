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

use core::{bytes_to_json, string_to_json, DarkskyConverter};
mod core {
    //! These items are common to other modules in the backend.
    use super::*;
    use crate::entities::DailyHistory;
    use chrono::NaiveDate;
    use serde_json::{json, Value};

    pub(super) trait DarkskyConverter {
        fn into_json(history: &DailyHistory) -> Value;
        fn from_json(alias: &str, date: &NaiveDate, history: &Value) -> Result<DailyHistory>;
    }
    impl DarkskyConverter for DailyHistory {
        fn into_json(history: &DailyHistory) -> Value {
            let mut json = json!({
                "daily": json!({
                    "data": [
                    ]
                })
            });
            let daily_data = json["daily"]["data"].as_array_mut().unwrap();
            daily_data.push(json!({
                "apparentTemperatureHigh": null,
                "apparentTemperatureHighTime": null,
                "apparentTemperatureLow": null,
                "apparentTemperatureLowTime": null,
                "apparentTemperatureMax": null,
                "apparentTemperatureMaxTime": null,
                "apparentTemperatureMin": null,
                "apparentTemperatureMinTime": null,
                "cloudCover": history.cloud_cover,
                "dewPoint": history.dew_point,
                "humidity": history.humidity,
                "icon": null,
                "moonPhase": history.moon_phase,
                "precipIntensity": null,
                "precipIntensityMax": null,
                "precipIntensityMaxTime": null,
                "precipProbability": null,
                "precipType": null,
                "pressure": null,
                "summary": history.summary,
                "sunriseTime": history.sunrise_time,
                "sunsetTime": history.sunset_time,
                "temperatureHigh": history.temperature_high,
                "temperatureHighTime": history.temperature_high_time,
                "temperatureLow": history.temperature_low,
                "temperatureLowTime": history.temperature_low_time,
                "temperatureMax": history.temperature_max,
                "temperatureMaxTime": history.temperature_max_time,
                "temperatureMin": history.temperature_min,
                "temperatureMinTime": history.temperature_min_time,
                "time": (history.date - NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()).num_seconds(),
                "uvIndex": history.uv_index,
                "uvIndexTime": history.uv_index_time,
                "visibility": null,
                "windBearing": history.wind_bearing,
                "windGust": history.wind_gust,
                "windGustTime": history.wind_gust_time,
                "windSpeed": history.wind_speed,
            }));
            json
        }

        fn from_json(alias: &str, date: &NaiveDate, json: &Value) -> Result<DailyHistory> {
            let json = &json["daily"]["data"][0];
            if json.is_object() {
                macro_rules! to_value {
                    ($name:literal, $fn:expr) => {
                        json.get($name).map_or(None, $fn)
                    };
                }
                Ok(DailyHistory {
                    location_id: alias.to_string(),
                    date: date.clone(),
                    temperature_high: to_value!("temperatureHigh", |v| v.as_f64()),
                    temperature_high_time: to_value!("temperatureHighTime", |v| v.as_i64()),
                    temperature_low: to_value!("temperatureLow", |v| v.as_f64()),
                    temperature_low_time: to_value!("temperatureLowTime", |v| v.as_i64()),
                    temperature_max: to_value!("temperatureMax", |v| v.as_f64()),
                    temperature_max_time: to_value!("temperatureMaxTime", |v| v.as_i64()),
                    temperature_min: to_value!("temperatureMin", |v| v.as_f64()),
                    temperature_min_time: to_value!("temperatureMinTime", |v| v.as_i64()),
                    wind_speed: to_value!("windSpeed", |v| v.as_f64()),
                    wind_gust: to_value!("windGust", |v| v.as_f64()),
                    wind_gust_time: to_value!("windGustTime", |v| v.as_i64()),
                    wind_bearing: to_value!("windBearing", |v| v.as_i64()),
                    cloud_cover: to_value!("cloudCover", |v| v.as_f64()),
                    uv_index: to_value!("uvIndex", |v| v.as_i64()),
                    uv_index_time: to_value!("uvIndexTime", |v| v.as_i64()),
                    summary: to_value!("summary", |v| match v {
                        Value::String(s) => Some(s.to_string()),
                        _ => None,
                    }),
                    humidity: to_value!("humidity", |v| v.as_f64()),
                    dew_point: to_value!("dewPoint", |v| v.as_f64()),
                    sunrise_time: to_value!("sunriseTime", |v| v.as_i64()),
                    sunset_time: to_value!("sunsetTime", |v| v.as_i64()),
                    moon_phase: to_value!("moonPhase", |v| v.as_f64()),
                })
            } else {
                let reason = format!("{}: Did not find daily history for {}.", alias, date);
                Err(Error::from(reason))
            }
        }
    }

    pub(super) fn string_to_json(string: &str) -> Result<Value> {
        bytes_to_json(string.as_bytes())
    }
    pub(super) fn bytes_to_json(bytes: &[u8]) -> Result<Value> {
        let reader_result: std::result::Result<Value, serde_json::Error> = serde_json::from_reader(bytes);
        match reader_result {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::from(err.to_string())),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn converter() {
            let alias = "testcase";
            let date = NaiveDate::from_ymd_opt(2023, 8, 31).unwrap();
            let history = DailyHistory {
                location_id: alias.to_string(),
                date,
                temperature_high: Some(51.12),
                temperature_high_time: Some(1396383420),
                temperature_low: Some(38.79),
                temperature_low_time: Some(1396446360),
                temperature_max: Some(51.12),
                temperature_max_time: Some(1396383420),
                temperature_min: Some(42.34),
                temperature_min_time: Some(1396360320),
                wind_speed: Some(0.78),
                wind_gust: Some(6.83),
                wind_gust_time: Some(1396350000),
                wind_bearing: Some(184),
                cloud_cover: Some(0.99),
                uv_index: Some(4),
                uv_index_time: Some(1396383300),
                summary: Some("summary".to_string()),
                humidity: Some(0.9),
                dew_point: Some(43.9),
                sunrise_time: Some(1396360320),
                sunset_time: Some(1396406400),
                moon_phase: Some(0.08),
            };
            let json = DailyHistory::into_json(&history);
            let testcase = &json["daily"]["data"][0];
            macro_rules! to_value {
                ($name:literal, $fn:expr) => {
                    testcase.get($name).map_or(None, $fn)
                };
            }
            assert_eq!(to_value!("temperatureHigh", |v| v.as_f64()), history.temperature_high);
            assert_eq!(to_value!("temperatureHighTime", |v| v.as_i64()), history.temperature_high_time);
            assert_eq!(to_value!("temperatureLow", |v| v.as_f64()), history.temperature_low);
            assert_eq!(to_value!("temperatureLowTime", |v| v.as_i64()), history.temperature_low_time);
            assert_eq!(to_value!("temperatureMax", |v| v.as_f64()), history.temperature_max);
            assert_eq!(to_value!("temperatureMaxTime", |v| v.as_i64()), history.temperature_max_time);
            assert_eq!(to_value!("temperatureMin", |v| v.as_f64()), history.temperature_min);
            assert_eq!(to_value!("temperatureMinTime", |v| v.as_i64()), history.temperature_min_time);
            assert_eq!(to_value!("windBearing", |v| v.as_i64()), history.wind_bearing);
            assert_eq!(to_value!("windGust", |v| v.as_f64()), history.wind_gust);
            assert_eq!(to_value!("windGustTime", |v| v.as_i64()), history.wind_gust_time);
            assert_eq!(to_value!("windSpeed", |v| v.as_f64()), history.wind_speed);
            assert_eq!(to_value!("uvIndex", |v| v.as_i64()), history.uv_index);
            assert_eq!(to_value!("uvIndexTime", |v| v.as_i64()), history.uv_index_time);
            assert_eq!(to_value!("cloudCover", |v| v.as_f64()), history.cloud_cover);
            let testcase_summary = to_value!("summary", |v| v.as_str()).unwrap();
            assert_eq!(Some(testcase_summary.to_string()), history.summary);
            assert_eq!(to_value!("humidity", |v| v.as_f64()), history.humidity);
            assert_eq!(to_value!("dewPoint", |v| v.as_f64()), history.dew_point);
            assert_eq!(to_value!("sunriseTime", |v| v.as_i64()), history.sunrise_time);
            assert_eq!(to_value!("sunsetTime", |v| v.as_i64()), history.sunset_time);
            assert_eq!(to_value!("moonPhase", |v| v.as_f64()), history.moon_phase);
            let testcase = DailyHistory::from_json(alias, &date, &json).unwrap();
            assert_eq!(testcase.location_id, alias);
            assert_eq!(testcase.date, date);
            assert_eq!(testcase.temperature_high, history.temperature_high);
            assert_eq!(testcase.temperature_high_time, history.temperature_high_time);
            assert_eq!(testcase.temperature_low, history.temperature_low);
            assert_eq!(testcase.temperature_low_time, history.temperature_low_time);
            assert_eq!(testcase.temperature_max, history.temperature_max);
            assert_eq!(testcase.temperature_max_time, history.temperature_max_time);
            assert_eq!(testcase.temperature_min, history.temperature_min);
            assert_eq!(testcase.temperature_min_time, history.temperature_min_time);
            assert_eq!(testcase.wind_bearing, history.wind_bearing);
            assert_eq!(testcase.wind_gust, history.wind_gust);
            assert_eq!(testcase.wind_gust_time, history.wind_gust_time);
            assert_eq!(testcase.wind_speed, history.wind_speed);
            assert_eq!(testcase.uv_index, history.uv_index);
            assert_eq!(testcase.uv_index_time, history.uv_index_time);
            assert_eq!(testcase.cloud_cover, history.cloud_cover);
            assert_eq!(testcase.summary, history.summary);
            assert_eq!(testcase.humidity, history.humidity);
            assert_eq!(testcase.dew_point, history.dew_point);
            assert_eq!(testcase.sunrise_time, history.sunrise_time);
            assert_eq!(testcase.sunset_time, history.sunset_time);
            assert_eq!(testcase.moon_phase, history.moon_phase);
        }
    }

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
                let target = self.0.join(source.file_name().unwrap().to_str().unwrap());
                if let Err(err) = fs::copy(source, &target) {
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
