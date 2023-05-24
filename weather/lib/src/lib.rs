//! A RUST based weather data sample implementation.
//!
//! This implementation is loosely base on a `Python` project I created several years ago. When
//! I started the `Python` project I wanted to chart historical weather information for different
//! areas we were interested in spending the winter. The idea of building a CLI based on the
//! original weather data implementation seemed like a fun journey.
//!
//! # History
//!
//! My intent was to build something to continue with RUST and after going through the various
//! tutorials. The `Python` weather data project is based on the ***Dark Sky*** project data.
//! Unfortunately the API was purchased by Apple and is no longer publicly available (or at least
//! free) but I had collected years of data for a dozen or more sites.
//!
//! # Architecture Overview
//!
//! The crate consists of two modules.
//!
//! * The weather data API and data objects (domain)
//! * The API that reads weather data (data).
//!
//! The two (2) modules are loosely coupled. The [weather_data] function is currently the
//! assembler of the modules.
//!
mod data;
mod domain;

use std::path::PathBuf;

pub use domain::{
    DailyHistories, DailyHistory, DailyHistoryQuery, Error, HistoryDates, HistoryRange, HistorySummary, Location,
    LocationDailyHistories, LocationHistoryDates, LocationHistorySummaries, LocationQuery, Locations, Result,
    WeatherData,
};

pub fn weather_data(files_path: &PathBuf) -> Result<WeatherData> {
    let root_pathname = files_path.as_path().display().to_string();
    let data_api = data::get_filesapi(&root_pathname)?;
    Ok(WeatherData::new(data_api))
}
