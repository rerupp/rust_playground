//! The implementations of weather data.

pub(crate) mod db;
pub(crate) mod filesys;

use crate::entities::{DailyHistories, DataCriteria, DateRange, History, HistoryDates, HistorySummaries, Location};
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
impl From<String> for Error {
    /// Create an API error from a string.
    fn from(error: String) -> Self {
        Error(error)
    }
}
impl From<&str> for Error {
    /// Create an API error from a string reference.
    fn from(error: &str) -> Self {
        Error(error.to_string())
    }
}

/// The `API` common to all the backend implementations.
pub(crate) trait DataAdapter {
    /// Add weather data history for a location.
    ///
    /// # Arguments
    ///
    /// * `histories` has the location and histories to add.
    #[allow(unused_variables)]
    fn add_histories(&self, histories: &DailyHistories) -> Result<usize> {
        Err(Error::from("DataAdapter::add_histories(...) needs to be implemented!"))
    }
    /// Returns the daily weather data history for a location.
    ///
    /// # Arguments
    ///
    /// * `criteria` identifies what location should be used.
    /// * `history_range` specifies the date range that should be used.
    fn daily_histories(&self, location: Location, date_range: DateRange) -> Result<DailyHistories>;
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

pub(crate) mod config {
    //! Utilities to load application configurations from `TOML` files at runtime.
    use super::{Error, Result};
    use std::{
        fs::File,
        io::prelude::*,
        path::{Path, PathBuf},
    };
    use toml;

    /// The application settings type definition.
    type Settings = toml::map::Map<String, toml::Value>;

    /// A helper to mine applicatin settings.
    macro_rules! setting {
        ($settings:expr, $key:expr, $default:expr, $fn:expr) => {
            match $settings.get($key) {
                // Some(value) => value.as_str().map_or_else(|| $default, |v| v.to_string()),
                Some(value) => value.as_str().map_or_else(|| $default, $fn),
                None => $default,
            }
        };
    }

    /// The weather application settings.
    #[derive(Debug)]
    pub(crate) struct WeatherSettings {
        pub(crate) uri: String,
        pub(crate) key: String,
    }
    impl WeatherSettings {
        /// The default Visual Crossing endpoint that will be called.
        pub(super) const DEFAULT_URI: &str =
            "https://weather.visualcrossing.com/VisualCrossingWebServices/rest/services/timeline";

        /// The environment variable name that contains the Visual Crossing API key.
        const API_KEY: &str = "VISUAL_CROSSING_KEY";

        /// Create a new instance of the settings from the configuration file.
        ///
        /// If the configuration file does not exist default values will be used.
        pub(crate) fn new() -> Self {
            let path = PathBuf::from("weather.toml");
            if !path.exists() {
                Default::default()
            } else {
                match load(&path) {
                    Ok(settings) => Self::initialize(settings),
                    Err(err) => {
                        log::error!("{} Using defaults.", err);
                        Default::default()
                    }
                }
            }
        }

        /// Initialze the weather application settings based on what was read from the configuration file.
        ///
        /// # Arguments
        ///
        /// * `settings` has the weather settings mined from the configuration file.
        fn initialize(settings: Settings) -> Self {
            let default: WeatherSettings = Default::default();
            let uri = setting!(settings, "uri", default.uri, |v| v.to_string());
            let key = setting!(settings, "key", default.key, |v| v.to_string());
            WeatherSettings { uri, key }
        }
    }
    impl TryFrom<&str> for WeatherSettings {
        type Error = Error;
        /// Load the weather application configuration from a file.
        ///
        /// # Arguments
        ///
        /// * `filename` is the file that contains configuration settings.
        fn try_from(filename: &str) -> std::result::Result<Self, Self::Error> {
            let settings = load(PathBuf::from(filename).as_path())?;
            Ok(Self::initialize(settings))
        }
    }
    impl Default for WeatherSettings {
        /// Initialize the settings using default values.
        ///
        /// The Visual Crossing API key will be retrieved from the process environment using the [WeatherSettings] `API_KEY`.
        fn default() -> Self {
            log::info!("Using default configuration.");
            let key = match std::env::var(Self::API_KEY) {
                Ok(key) => key,
                Err(_) => Default::default(),
            };
            Self { uri: Self::DEFAULT_URI.to_string(), key }
        }
    }

    /// The settings file loader.
    ///
    /// # Arguments
    ///
    /// * `path` is the configuration file pathname.
    fn load(path: &Path) -> Result<Settings> {
        match path.is_file() {
            true => match File::open(path) {
                Ok(file) => parse_file(path, file),
                Err(err) => {
                    let reason = format!("Could not open {} ({}).", path.display(), err);
                    Err(Error::from(reason))
                }
            },
            false => {
                let what = if path.exists() { "does not exist" } else { "must be a file" };
                let reason = format!("{} {}.", path.display(), what);
                Err(Error::from(reason))
            }
        }
    }

    /// Parse the settings configuration file.
    ///
    /// # Arguments
    ///
    /// * `path` is the configuration file pathname.
    /// * `file` is used to read the configuration setting contents.
    fn parse_file(path: &Path, mut file: File) -> Result<Settings> {
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => match toml::from_str::<toml::Table>(&contents) {
                Ok(config) => Ok(config),
                Err(err) => {
                    let reason = format!("Failed to parse {} ({})", path.display(), err);
                    Err(Error::from(reason))
                }
            },
            Err(err) => {
                let reason = format!("Could not read {} ({})", path.display(), err);
                Err(Error::from(reason))
            }
        }
    }

    #[cfg(test)]
    use super::testlib;

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::env;
        use testlib::test_resources;

        #[test]
        fn weather() {
            env::remove_var(WeatherSettings::API_KEY);
            let testcase = WeatherSettings::new();
            assert_eq!(testcase.uri, WeatherSettings::DEFAULT_URI);
            assert!(testcase.key.is_empty());
            env::set_var(WeatherSettings::API_KEY, "testcase key");
            let testcase = WeatherSettings::new();
            assert_eq!(testcase.uri, WeatherSettings::DEFAULT_URI);
            assert_eq!(testcase.key, "testcase key");
            env::remove_var(WeatherSettings::API_KEY);
            let test_resource = test_resources().join("config").join("weather.toml");
            let testcase = WeatherSettings::try_from(test_resource.as_path().display().to_string().as_str()).unwrap();
            assert_eq!(testcase.uri, "vs endpoint");
            assert_eq!(testcase.key, "some api key");
        }

        #[test]
        fn admin() {}
    }
}

/// Calls the [TimelineClient] to get weather history for a location.
/// 
/// # Arguments
/// 
/// * `location` identifies what weather history will be retrieved.
/// * `dates` are the weather history dates of interest.
pub(crate) fn get_weather_history(location: &Location, dates: &DateRange) -> Result<Vec<History>> {
    let stopwatch = toolslib::stopwatch::StopWatch::start_new();
    let client = TimelineClient::new()?;
    let histories = client.get(&location, &dates)?;
    log::trace!("Visual Crossing call {}", &stopwatch);
    Ok(histories)
}

pub(self) use visual_crossing::TimelineClient;
pub(in crate::backend) mod visual_crossing {
    //! The Visual Crossing weather data services client.

    use super::*;
    use crate::entities::History;
    use chrono::NaiveDateTime;
    use config::WeatherSettings;
    use reqwest::{
        blocking::{Client, Request, Response},
        StatusCode, Url,
    };
    use serde::Deserialize;
    use serde_json::Value;

    #[derive(Debug)]
    /// The HTTP client metadata.
    pub(in crate::backend) struct TimelineClient {
        /// The client instance.
        client: Client,
        /// The Visual Crossing base URL.
        url: Url,
        /// The Visual Crossing API key.
        api_key: String,
    }
    impl TimelineClient {
        /// Creates a new instance of the HTTP client metadata.
        ///
        /// # Arguments
        ///
        /// * `endpoint` is the Visual Crossing timeline endpoint.
        /// * `api_key` is the Visual Crossing API key.
        pub(in crate::backend) fn new() -> Result<Self> {
            let weather_settings = WeatherSettings::new();
            Self::try_from(&weather_settings)
        }
        /// Uses the Visual Crossing timeline API to get history for a location.
        ///
        /// # Arguments
        ///
        /// * `location` is whose history will be queried.
        /// * `date_range` is the history dates to query.
        pub(in crate::backend) fn get(&self, location: &Location, date_range: &DateRange) -> Result<Vec<History>> {
            let request = self.create_request(&location.latitude, &location.longitude, &date_range)?;
            match self.client.execute(request) {
                Ok(response) => {
                    let json = self.process_response(&location.alias, response)?;
                    to_histories(location, json)
                }
                Err(err) => {
                    let reason = format!("Http error for {} ({}).", self.url, err);
                    Err(Error::from(reason))
                }
            }
        }
        /// Creates the Visual Crossing timeline URL to query weather history.
        ///
        /// # Arguments
        ///
        /// * `latitude` is the locations latitude.
        /// * `longitude` is the locations longitude.
        /// * `date_range` identifies the history dates of interest.
        fn create_request(&self, latitude: &str, longitude: &str, date_range: &DateRange) -> Result<Request> {
            // add the location
            let location = format!("{},{}", latitude, longitude);
            match self.url.join(&location) {
                Ok(mut url) => {
                    // add in the date range
                    let (from, to) = date_range.as_iso8601();
                    if date_range.is_one_day() {
                        url.path_segments_mut().unwrap().push(&from);
                    } else {
                        url.path_segments_mut().unwrap().push(&from).push(&to);
                    }
                    // add the query parameters
                    let builder =
                        self.client.get(url).query(&[("unitGroup", "us"), ("include", "days"), ("key", &self.api_key)]);
                    // build the request
                    match builder.build() {
                        Ok(request) => Ok(request),
                        Err(err) => {
                            let reason = format!("Error building request ({})", err);
                            Err(Error::from(reason))
                        }
                    }
                }
                Err(err) => {
                    let reason = format!("Error adding URL location {} ({})", location, err);
                    Err(Error::from(reason))
                }
            }
        }
        /// Process the Visual Crossing HTTP response.
        ///
        /// # Arguments
        ///
        /// * `alias` is the locations alias name.
        /// * `response` is the result of calling the Visual Crossing timeline API.
        fn process_response(&self, alias: &str, response: Response) -> Result<Value> {
            match response.status() {
                StatusCode::OK => match response.bytes() {
                    Ok(body) => match serde_json::from_slice(&body) {
                        Ok(json) => Ok(json),
                        Err(err) => {
                            let reason = format!("Error parsing response body ({}).", err);
                            Err(Error::from(reason))
                        }
                    },
                    Err(err) => {
                        let reason = format!("Error getting response text ({}).", err);
                        Err(Error::from(reason))
                    }
                },
                StatusCode::TOO_MANY_REQUESTS => Err(Error::from("Too many requests today.")),
                StatusCode::NOT_FOUND => Err(Error::from(format!("History not found for '{}'.", alias))),
                StatusCode::UNAUTHORIZED => Err(Error::from("API key was not accepted.")),
                _ => Err(Error::from(format!("HTTP error {}", response.status().as_str()))),
            }
        }
    }
    impl TryFrom<&WeatherSettings> for TimelineClient {
        type Error = Error;

        fn try_from(settings: &WeatherSettings) -> result::Result<Self, Self::Error> {
            if settings.uri.is_empty() {
                Err(Error::from("The client endpoint cannot be empty."))
            } else if settings.key.is_empty() {
                Err(Error::from("The api_key cannot be empty."))
            } else {
                // make sure there is a tailing slash so the Url join will work right
                let endpoint =
                    if settings.uri.ends_with("/") { settings.uri.to_string() } else { format!("{}/", settings.uri) };
                match Url::parse(&endpoint) {
                    Ok(url) => Ok(Self { client: Client::new(), url, api_key: settings.key.to_string() }),
                    Err(err) => {
                        let reason = format!("Error parsing URL='{}' ({})", endpoint, err);
                        Err(Error::from(reason))
                    }
                }
            }
        }
    }

    /// Consummes the Visual Crossing daily timeline response and creates a collection of [History].
    ///
    /// # Arguments
    ///
    /// * `location` is whose history is being created.
    /// * `json` is the API response body.
    fn to_histories(location: &Location, mut json: Value) -> Result<Vec<History>> {
        match json["days"].take() {
            Value::Array(days) => {
                let mut histories = vec![];
                for timeline_day in days {
                    match to_history(&location.alias, timeline_day) {
                        Ok(history) => histories.push(history),
                        #[cfg(not(test))]
                        Err(err) => log::error!("{}", err),
                        #[cfg(test)]
                        Err(err) => eprintln!("{}", err),
                    }
                }
                Ok(histories)
            }
            Value::Null => {
                let reason = format!("Did not find 'days' in JSON for {}", location.alias);
                Err(Error::from(reason))
            }
            _ => {
                let reason = format!("Expected 'days' to be an array of objects for {}", location.alias);
                Err(Error::from(reason))
            }
        }
    }

    /// Consummes the Visual Crossing daily weather data object.
    ///
    /// # Arguments
    ///
    /// * `alias` is the location alias name.
    /// * `timeline_day` is the weather data object.
    fn to_history(alias: &str, timeline_day: Value) -> Result<History> {
        // there should always be a date in the response
        match timeline_day.get("datetime") {
            Some(date) => {
                let date = date.to_string();
                match serde_json::from_value::<TimelineDay>(timeline_day) {
                    Ok(day) => Ok(day.into_history(alias)),
                    Err(err) => {
                        let reason = format!("{} {}: error converting timeline to history ({}).", alias, date, err);
                        Err(Error::from(reason))
                    }
                }
            }
            None => Err(Error::from("Did not find 'date' in timeline day response.")),
        }
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    /// Defines the fields of interest from the weather data daily object.
    struct TimelineDay {
        /// The date associated with the history.
        datetime: String,
        /// The high temperature.
        tempmax: Option<f64>,
        /// The low temperature.
        tempmin: Option<f64>,
        /// The mean temperature.
        temp: Option<f64>,
        /// The dew point.
        dew: Option<f64>,
        /// The humidity.
        humidity: Option<f64>,
        /// The amount of rain.
        precip: Option<f64>,
        /// The chance of rain.
        precipprob: Option<f64>,
        /// The type  of rain (this be null if it's not rainy day).
        preciptype: Option<Vec<String>>,
        /// The highest wind speed recorded.
        windgust: Option<f64>,
        /// The wind speed.
        windspeed: Option<f64>,
        /// The wind direction in degrees.
        winddir: Option<f64>,
        /// The barometric pressure in millibars.
        pressure: Option<f64>,
        /// The percent of sky covered by clouds.
        cloudcover: Option<f64>,
        /// The visibility distance.
        visibility: Option<f64>,
        /// The level of ultaviolet exposure.
        uvindex: Option<f64>,
        /// The time when the sun rises.
        sunriseEpoch: Option<i64>,
        /// The time when the sun sets.
        sunsetEpoch: Option<i64>,
        /// The moons phase.
        moonphase: Option<f64>,
        /// The description of weather for the day.
        description: Option<String>,
    }
    impl TimelineDay {
        /// Convert the visual crossing timeline day into [History].
        ///
        /// # Arguments
        ///
        /// * `alias` is the location alias name.
        fn into_history(self, alias: &str) -> History {
            History {
                alias: alias.to_string(),
                date: toolslib::date_time::parse_date(&self.datetime).map_or(Default::default(), |d| d),
                temperature_high: self.tempmax,
                temperature_low: self.tempmin,
                temperature_mean: self.temp,
                dew_point: self.dew,
                humidity: self.humidity.map_or(Default::default(), |h| Some(h / 100.0)),
                /// there % scale seems to b 0.0 to 100.0
                precipitation_chance: self.precipprob.map_or(Default::default(), |p| Some(p / 100.0)),
                precipitation_type: self.preciptype.map_or(Default::default(), |t| Some(t.join(" "))),
                precipitation_amount: self.precip,
                wind_speed: self.windspeed,
                wind_gust: self.windgust,
                wind_direction: self.winddir.map_or(Default::default(), |d| Some(d.round() as i64)),
                cloud_cover: self.cloudcover.map_or(Default::default(), |c| Some(c / 100.0)),
                pressure: self.pressure,
                uv_index: self.uvindex,
                sunrise: self.sunriseEpoch.map_or(Default::default(), |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                sunset: self.sunsetEpoch.map_or(Default::default(), |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                moon_phase: self.moonphase,
                visibility: self.visibility,
                description: self.description,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use toolslib::date_time::get_date;

        #[test]
        fn client() {
            // let testcase = HttpClient::new("https://some/base/uri", "KEY").unwrap();
            let settings = WeatherSettings { uri: "https://some/base/uri".to_string(), key: "KEY".to_string() };
            let testcase = TimelineClient::try_from(&settings).unwrap();
            assert_eq!(testcase.url.as_str(), "https://some/base/uri/");
            assert_eq!(testcase.api_key, "KEY");
            // let testcase = HttpClient::new("https://some/base/uri/", "YEK").unwrap();
            let settings = WeatherSettings { uri: "https://some/base/uri".to_string(), key: "YEK".to_string() };
            let testcase = TimelineClient::try_from(&settings).unwrap();
            assert_eq!(testcase.url.as_str(), "https://some/base/uri/");
            assert_eq!(testcase.api_key, "YEK");
            assert!(TimelineClient::try_from(&WeatherSettings {
                uri: "https:/uri".to_string(),
                key: Default::default()
            })
            .is_err());
            assert!(
                TimelineClient::try_from(&WeatherSettings { uri: Default::default(), key: "KEY".to_string() }).is_err()
            );
            assert!(TimelineClient::try_from(&WeatherSettings {
                uri: "::/bad/uri".to_string(),
                key: "KEY".to_string()
            })
            .is_err());
        }

        #[test]
        fn request() {
            let client = TimelineClient::try_from(&WeatherSettings {
                uri: "https://some/base/uri".to_string(),
                key: "KEY".to_string(),
            })
            .unwrap();
            let date_range = DateRange::new(get_date(2023, 9, 1), get_date(2023, 9, 1));
            let testcase = client.create_request("32.22", "-110.97", &date_range).unwrap();
            assert_eq!(
                testcase.url().as_str(),
                "https://some/base/uri/32.22,-110.97/2023-09-01?unitGroup=us&include=days&key=KEY"
            );
            let date_range = DateRange::new(get_date(2023, 9, 1), get_date(2023, 9, 7));
            let testcase = client.create_request("32.22", "-110.97", &date_range).unwrap();
            assert_eq!(
                testcase.url().as_str(),
                "https://some/base/uri/32.22,-110.97/2023-09-01/2023-09-07?unitGroup=us&include=days&key=KEY"
            );
        }

        #[allow(unused)]
        // #[test]
        fn call() {
            let location = Location {
                name: "Foothills, AZ".to_string(),
                alias: "foothills".to_string(),
                longitude: "-114.408".to_string(),
                latitude: "32.6526".to_string(),
                tz: "America/Phoenix".to_string(),
            };
            let settings = config::WeatherSettings::try_from("../weather.toml").unwrap();
            let client = TimelineClient::try_from(&settings).unwrap();
            let dates = DateRange::new(get_date(2023, 9, 1), get_date(2023, 9, 7));
            let history = client.get(&location, &dates).unwrap();
        }
    }
}

pub(in crate::backend) mod history {
    //! Convert [History] to and from a `JSON` byte stream.

    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde::{Deserialize, Serialize};

    /// This is the structure used to serialize and deserialize [History].
    #[derive(Debug, Deserialize, Serialize)]
    struct HistoryDoc {
        /// The histories date.
        date: NaiveDate,
        /// The time in seconds (UTC) the sun rises.
        sunrise: Option<i64>,
        /// The time in seconds (UTC) the sun sets.
        sunset: Option<i64>,
        /// The phase of the moon.
        moon: Option<f64>,
        /// The maximum temperature for the date.
        tempmax: Option<f64>,
        /// The minimum temperature for the date.
        tempmin: Option<f64>,
        /// The temperature mean for the date.
        tempmean: Option<f64>,
        /// The dew point.
        dewpoint: Option<f64>,
        /// The chance of rain.
        precipprob: Option<f64>,
        /// The amount of precipitation.
        precip: Option<f64>,
        /// A description of the type of precipitation.
        preciptype: Option<String>,
        /// The humidity.
        humidity: Option<f64>,
        /// The atmospheric pressue in millibars.
        pressure: Option<f64>,
        /// The percent of cloud cover.
        cloud: Option<f64>,
        /// The UV index.
        uv: Option<f64>,
        /// The visibility in miles.
        vis: Option<f64>,
        /// The wind speed in miles per hour.
        wind: Option<f64>,
        /// The maximum wind gust speed in miles per hour.
        windgust: Option<f64>,
        /// The predominant wind speed.
        winddir: Option<i64>,
        /// A summary description of the weather.
        summary: Option<String>,
    }
    impl HistoryDoc {
        /// Convert the deserialized history to a [History] instance.
        ///
        /// # Arguments
        ///
        /// * `alias` is the location alias name.
        fn to_history(self, alias: &str) -> History {
            History {
                alias: alias.to_string(),
                date: self.date,
                temperature_high: self.tempmax,
                temperature_low: self.tempmin,
                temperature_mean: self.tempmean,
                dew_point: self.dewpoint,
                humidity: self.humidity,
                precipitation_chance: self.precipprob,
                precipitation_type: self.preciptype,
                precipitation_amount: self.precip,
                wind_speed: self.wind,
                wind_gust: self.windgust,
                wind_direction: self.winddir,
                cloud_cover: self.cloud,
                pressure: self.pressure,
                uv_index: self.uv,
                sunrise: self.sunrise.map_or(None, |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                sunset: self.sunset.map_or(None, |ts| NaiveDateTime::from_timestamp_opt(ts, 0)),
                moon_phase: self.moon,
                visibility: self.vis,
                description: self.summary,
            }
        }
    }
    impl From<&History> for HistoryDoc {
        /// Convert [History] into the document that can be serialized and deserialized.
        fn from(history: &History) -> Self {
            Self {
                date: history.date.clone(),
                sunrise: history.sunrise.map_or(None, |ts| Some(ts.timestamp())),
                sunset: history.sunset.map_or(None, |ts| Some(ts.timestamp())),
                moon: history.moon_phase,
                tempmax: history.temperature_high,
                tempmin: history.temperature_low,
                tempmean: history.temperature_mean,
                dewpoint: history.dew_point,
                precipprob: history.precipitation_chance,
                precip: history.precipitation_amount,
                preciptype: history.precipitation_type.clone(),
                humidity: history.humidity,
                pressure: history.pressure,
                cloud: history.cloud_cover,
                uv: history.uv_index,
                vis: history.visibility,
                wind: history.wind_speed,
                windgust: history.wind_gust,
                winddir: history.wind_direction,
                summary: history.description.clone(),
            }
        }
    }

    /// Convert [History] into a sequence of bytes.
    ///
    /// # Arguments
    ///
    /// * `history` will be converted to a sequence of bytes.
    pub(in crate::backend) fn to_bytes(history: &History) -> Result<Vec<u8>> {
        match serde_json::to_string(&HistoryDoc::from(history)) {
            Ok(json) => Ok(json.into_bytes()),
            Err(err) => {
                let reason = format!("History serialize error for {} on {} ({})", history.alias, history.date, err);
                Err(Error::from(reason))
            }
        }
    }

    #[allow(unused)]
    /// Convert a sequence of bytes into a [History]e.
    ///
    /// # Arguments
    ///
    /// * `alias` is the locations alias name.
    /// * `bytes` will be converted to a [History] instance.
    pub(in crate::backend) fn from_bytes(alias: &str, bytes: &[u8]) -> Result<History> {
        match serde_json::from_slice::<HistoryDoc>(bytes) {
            Ok(history_doc) => Ok(history_doc.to_history(alias)),
            Err(err) => {
                let reason = format!("Yikes... Error creating History for {} ({})", alias, err);
                Err(Error::from(reason))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use toolslib::date_time::{get_date, get_time};

        #[test]
        fn json() {
            let alias = "test";
            let history = History {
                alias: alias.to_string(),
                date: get_date(2023, 9, 12),
                temperature_high: Some(77.0),
                temperature_low: Some(56.0),
                temperature_mean: Some(65.8),
                dew_point: Some(60.3),
                humidity: Some(43.0),
                precipitation_chance: Some(8.0),
                precipitation_type: Some("rain".to_string()),
                precipitation_amount: Some(0.1),
                wind_speed: Some(6.0),
                wind_gust: Some(8.0),
                wind_direction: Some(337),
                cloud_cover: Some(7.3),
                pressure: Some(30.05),
                uv_index: Some(5.0),
                sunrise: Some(NaiveDateTime::new(get_date(2023, 9, 12), get_time(13, 45, 0))),
                sunset: Some(NaiveDateTime::new(get_date(2023, 9, 13), get_time(2, 28, 0))),
                moon_phase: Some(0.8),
                visibility: Some(10.0),
                description: Some("Sun and clouds mixed.".to_string()),
            };
            let json = to_bytes(&history).unwrap();
            let testcase = from_bytes(alias, json.as_slice()).unwrap();
            assert_eq!(history.alias, testcase.alias);
            assert_eq!(history.date, testcase.date);
            assert_eq!(history.temperature_high, testcase.temperature_high);
            assert_eq!(history.temperature_low, testcase.temperature_low);
            assert_eq!(history.temperature_mean, testcase.temperature_mean);
            assert_eq!(history.dew_point, testcase.dew_point);
            assert_eq!(history.humidity, testcase.humidity);
            assert_eq!(history.precipitation_chance, testcase.precipitation_chance);
            assert_eq!(history.precipitation_type, testcase.precipitation_type);
            assert_eq!(history.precipitation_amount, testcase.precipitation_amount);
            assert_eq!(history.wind_speed, testcase.wind_speed);
            assert_eq!(history.wind_gust, testcase.wind_gust);
            assert_eq!(history.wind_direction, testcase.wind_direction);
            assert_eq!(history.cloud_cover, testcase.cloud_cover);
            assert_eq!(history.pressure, testcase.pressure);
            assert_eq!(history.uv_index, testcase.uv_index);
            assert_eq!(history.sunrise, testcase.sunrise);
            assert_eq!(history.sunset, testcase.sunset);
            assert_eq!(history.moon_phase, testcase.moon_phase);
            assert_eq!(history.visibility, testcase.visibility);
            assert_eq!(history.description, testcase.description);
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
