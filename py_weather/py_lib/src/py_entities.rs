//! The weather data to Python class mappings.

use super::*;
use chrono::prelude::{NaiveDate, NaiveDateTime};
use std::path::PathBuf;
use weather_lib::prelude::{
    DailyHistories, DataCriteria, DateRange, History, HistoryDates, HistorySummaries, Location, LocationCriteria,
};

#[derive(Clone, Debug, Default)]
#[pyclass(name = "WeatherConfig", get_all, set_all)]
pub struct PyWeatherConfig {
    pub config_file: Option<PathBuf>,
    pub dirname: Option<PathBuf>,
    pub logfile: Option<PathBuf>,
    pub log_append: bool,
    pub log_level: usize,
    pub fs_only: bool,
}
#[pymethods]
impl PyWeatherConfig {
    #[new]
    #[pyo3(signature = (config_file=None, dirname=None, logfile=None, log_append=false, log_level=0, fs_only=false))]
    fn new(
        config_file: Option<PathBuf>,
        dirname: Option<PathBuf>,
        logfile: Option<PathBuf>,
        log_append: bool,
        log_level: usize,
        fs_only: bool) -> Self {
        Self {
            config_file,
            dirname,
            logfile,
            log_append,
            log_level,
            fs_only,
        }
    }
}

/// The `Python` data that comprises a location.
#[derive(Clone, Debug, Default)]
#[pyclass(name = "Location", get_all, set_all)]
pub struct PyLocation {
    /// The name of a location.
    pub name: String,
    /// A unique nickname of a location.
    pub alias: String,
    /// The location longitude.
    pub longitude: String,
    /// The location latitude.
    pub latitude: String,
    /// the location timezone.
    pub tz: String,
}
impl From<&Location> for PyLocation {
    fn from(location: &Location) -> Self {
        location.clone().into()
    }
}
impl From<Location> for PyLocation {
    fn from(location: Location) -> Self {
        Self {
            name: location.name,
            alias: location.alias,
            longitude: location.longitude,
            latitude: location.latitude,
            tz: location.tz,
        }
    }
}
impl From<PyLocation> for Location {
    fn from(location: PyLocation) -> Self {
        Self {
            name: location.name,
            alias: location.alias,
            longitude: location.longitude,
            latitude: location.latitude,
            tz: location.tz,
        }
    }
}
#[pymethods]
impl PyLocation {
    #[new]
    #[pyo3(signature = (name=None, alias=None, latitude=None, longitude=None, tz=None))]
    fn new(name: Option<String>, alias: Option<String>, latitude: Option<String>, longitude: Option<String>,
           tz: Option<String>) -> Self {
        Self {
            name: name.unwrap_or(Default::default()),
            alias: alias.unwrap_or(Default::default()),
            latitude: latitude.unwrap_or(Default::default()),
            longitude: longitude.unwrap_or(Default::default()),
            tz: tz.unwrap_or(Default::default()),
        }
    }
    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
    fn __copy__(&self) -> PyLocation {
        PyLocation::new(Some(self.name.clone()), Some(self.alias.clone()), Some(self.latitude.clone()),
                        Some(self.longitude.clone()), Some(self.tz.clone()))
    }
}

/// The weather history data.
#[derive(Clone, Debug, Default)]
#[pyclass(name = "History", get_all, set_all)]
pub struct PyHistory {
    /// The location alias name.
    pub alias: String,
    /// The history date.
    pub date: NaiveDate,
    /// The high temperature for the day.
    pub temperature_high: Option<f64>,
    /// The low temperature for the day.
    pub temperature_low: Option<f64>,
    /// The daily mean temperature.
    pub temperature_mean: Option<f64>,
    /// The dew point temperature.
    pub dew_point: Option<f64>,
    /// The relative humidity percentage.
    pub humidity: Option<f64>,
    /// The chance of rain during the day.
    pub precipitation_chance: Option<f64>,
    /// A short description of the type of rain.
    pub precipitation_type: Option<String>,
    /// The amount of precipitation for the day.
    pub precipitation_amount: Option<f64>,
    /// The daily wind speed.
    pub wind_speed: Option<f64>,
    /// The highest wind speed recorded for the day.
    pub wind_gust: Option<f64>,
    /// The general direction in degrees.
    pub wind_direction: Option<i64>,
    /// The percentage of sky covered by clouds.
    pub cloud_cover: Option<f64>,
    /// The daily atmospheric pressure expressed in millibars.
    pub pressure: Option<f64>,
    /// The level of ultraviolet exposure for the day.
    pub uv_index: Option<f64>,
    /// The local time when the sun comes up.
    pub sunrise: Option<NaiveDateTime>,
    /// The local time when the sun will set.
    pub sunset: Option<NaiveDateTime>,
    /// The moons phase between 0 and 1.
    pub moon_phase: Option<f64>,
    /// The distance that can be during the day.
    pub visibility: Option<f64>,
    /// A summary of the daily weather.
    pub description: Option<String>,
}
impl From<History> for PyHistory {
    fn from(history: History) -> Self {
        Self {
            alias: history.alias,
            date: history.date,
            temperature_high: history.temperature_high,
            temperature_low: history.temperature_low,
            temperature_mean: history.temperature_mean,
            dew_point: history.dew_point,
            humidity: history.humidity,
            precipitation_chance: history.precipitation_chance,
            precipitation_type: history.precipitation_type,
            precipitation_amount: history.precipitation_amount,
            wind_speed: history.wind_speed,
            wind_gust: history.wind_gust,
            wind_direction: history.wind_direction,
            cloud_cover: history.cloud_cover,
            pressure: history.pressure,
            uv_index: history.uv_index,
            sunrise: history.sunrise,
            sunset: history.sunset,
            moon_phase: history.moon_phase,
            visibility: history.visibility,
            description: history.description,
        }
    }
}
impl From<PyHistory> for History {
    fn from(location: PyHistory) -> Self {
        Self {
            alias: location.alias,
            date: location.date,
            temperature_high: location.temperature_high,
            temperature_low: location.temperature_low,
            temperature_mean: location.temperature_mean,
            dew_point: location.dew_point,
            humidity: location.humidity,
            precipitation_chance: location.precipitation_chance,
            precipitation_type: location.precipitation_type,
            precipitation_amount: location.precipitation_amount,
            wind_speed: location.wind_speed,
            wind_gust: location.wind_gust,
            wind_direction: location.wind_direction,
            cloud_cover: location.cloud_cover,
            pressure: location.pressure,
            uv_index: location.uv_index,
            sunrise: location.sunrise,
            sunset: location.sunset,
            moon_phase: location.moon_phase,
            visibility: location.visibility,
            description: location.description,
        }
    }
}
#[pymethods]
impl PyHistory {
    #[new]
    fn new() -> Self {
        Default::default()
    }
    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
}

/// A locations daily weather history.
#[derive(Clone, Debug, Default)]
#[pyclass(name = "DailyHistories", get_all, set_all)]
pub struct PyDailyHistories {
    /// The location metadata.
    pub location: PyLocation,
    /// The daily histories for a location.
    pub histories: Vec<PyHistory>,
}
impl From<DailyHistories> for PyDailyHistories {
    fn from(daily_histories: DailyHistories) -> Self {
        Self {
            location: daily_histories.location.into(),
            histories: daily_histories.histories.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<PyDailyHistories> for DailyHistories {
    fn from(daily_histories: PyDailyHistories) -> Self {
        Self {
            location: daily_histories.location.into(),
            histories: daily_histories.histories.into_iter().map(Into::into).collect(),
        }
    }
}
#[pymethods]
impl PyDailyHistories {
    #[new]
    fn __new__() -> Self {
        Default::default()
    }
    fn __str__(&self) -> String {
        let mut str = vec![];
        str.push("DailyHistories {".to_string());
        str.push(format!("  location: {:?}", self.location));
        str.push("  histories: [".to_string());
        self.histories.iter().for_each(|history| str.push(format!("  {:?}", history)));
        str.push("  ]".to_string());
        str.push("}".to_string());
        str.join("\n")
    }
}

/// Used by front-ends to identify locations.
#[derive(Clone, Debug)]
#[pyclass(name = "DataCriteria", get_all, set_all)]
pub struct PyDataCriteria {
    /// The locations of interest.
    pub filters: Vec<String>,
    /// If `true` the location filters will ignore case.
    pub icase: bool,
    /// If `true` locations will be sorted by name.
    pub sort: bool,
}
impl Default for PyDataCriteria {
    fn default() -> Self {
        Self { filters: Default::default(), icase: true, sort: true }
    }
}
impl From<PyDataCriteria> for DataCriteria {
    fn from(data_criteria: PyDataCriteria) -> Self {
        Self { filters: data_criteria.filters, icase: data_criteria.icase, sort: data_criteria.sort }
    }
}
#[pymethods]
impl PyDataCriteria {
    #[new]
    #[pyo3(signature = (filters=None, icase=true, sort=true))]
    fn new(filters: Option<Vec<String>>, icase: Option<bool>, sort: Option<bool>) -> Self {
        PyDataCriteria {
            filters: filters.unwrap_or(Default::default()),
            icase: icase.unwrap_or(true),
            sort: sort.unwrap_or(true),
        }
    }
    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
}

/// The container for a range of dates.
#[derive(Clone, Debug)]
#[pyclass(name = "DateRange", get_all, set_all)]
pub struct PyDateRange {
    /// The starting date of the range.
    pub start: NaiveDate,
    /// The inclusive end date of the range.
    pub end: NaiveDate,
}
impl From<DateRange> for PyDateRange {
    fn from(date_range: DateRange) -> Self {
        Self { start: date_range.start, end: date_range.end }
    }
}
impl From<PyDateRange> for DateRange {
    fn from(date_range: PyDateRange) -> Self {
        Self { start: date_range.start, end: date_range.end }
    }
}
#[pymethods]
impl PyDateRange {
    #[new]
    fn new(start: NaiveDate, end: NaiveDate) -> PyResult<Self> {
        match start > end {
            true => Err(pyo3::exceptions::PyValueError::new_err("from date greater than to date")),
            false => Ok(Self { start, end }),
        }
    }
    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
    fn __copy__(&self) -> PyDateRange {
        PyDateRange::new(self.start, self.end).unwrap()
    }
    fn __eq__(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
    fn contains(&self, date: NaiveDate) -> PyResult<bool> {
        Ok(self.start <= date && date <= self.end)
    }
}

#[derive(Debug, Default)]
#[pyclass(name = "LocationHistoryDates", get_all)]
pub struct PyLocationHistoryDates {
    /// The location metadata.
    pub location: PyLocation,
    /// The history dates metadata.
    pub history_dates: Vec<PyDateRange>,
}
impl From<HistoryDates> for PyLocationHistoryDates {
    fn from(history_dates: HistoryDates) -> Self {
        Self {
            location: history_dates.location.clone().into(),
            history_dates: history_dates.history_dates.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Default)]
#[pyclass(name = "HistorySummaries", get_all)]
/// A locations history summary.
pub struct PyHistorySummaries {
    location: PyLocation,
    /// The number of weather data histories available.
    count: usize,
    /// The overall size of weather data in bytes (may or may not be available).
    overall_size: Option<usize>,
    /// The size in bytes of weather data.
    raw_size: Option<usize>,
    /// The size in bytes of weather data in the backing store.
    store_size: Option<usize>,
}
impl From<HistorySummaries> for PyHistorySummaries {
    fn from(history_summaries: HistorySummaries) -> Self {
        Self {
            location: history_summaries.location.into(),
            count: history_summaries.count,
            overall_size: history_summaries.overall_size,
            raw_size: history_summaries.raw_size,
            store_size: history_summaries.store_size,
        }
    }
}
#[pymethods]
impl PyHistorySummaries {
    #[new]
    fn new() -> Self {
        Default::default()
    }
    fn ___str__(&self) -> String {
        format!("{:?}", self)
    }
}

/// The search criteria for locations.
#[derive(Clone, Debug, Default)]
#[pyclass(name = "LocationCriteria", get_all, set_all)]
pub struct PyLocationCriteria {
    /// The optional city name.
    pub name: Option<String>,
    /// The optional state name.
    pub state: Option<String>,
    /// Used to limit the result of the query.
    pub limit: usize,
}
impl From<PyLocationCriteria> for LocationCriteria {
    fn from(location_criteria: PyLocationCriteria) -> Self {
        Self { name: location_criteria.name, state: location_criteria.state, limit: location_criteria.limit }
    }
}

#[pymethods]
impl PyLocationCriteria {
    #[new]
    #[pyo3(signature = (name=None, state=None, limit=25))]
    fn new(name: Option<String>, state: Option<String>, limit: Option<usize>) -> Self {
        Self {
            name,
            state,
            limit: limit.unwrap_or(25),
        }
    }
    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
}
