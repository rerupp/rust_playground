//! The source of weather history for locations.

use std::fmt::Debug;
use super::*;
use backend::Config;
use entities::{DailyHistories, DateRange, Location};
use reqwest::{
    // use the blocking API since the rest client is async.
    blocking::{Client, Request, RequestBuilder},
    StatusCode,
    Url,
};
use rest_client::{RestClient, RestClientHandle, RestClientResult};
use timeline_client::TimelineClient;

mod rest_client;

mod timeline_client;

/// Creates a history client.
///
/// # Arguments
///
/// - `config` is the weather data configuration.
///
pub fn get(config: &Config) -> Result<Box<dyn HistoryClient>> {
    // currently there is only 1 client so just create it.
    match TimelineClient::new(config) {
        Ok(history_client) => Ok(Box::new(history_client)),
        Err(error) => Err(error),
    }
}

/// The internal API used to get location weather history.
///
pub trait HistoryClient: Debug {
    /// Execute the request to get history for a location.
    ///
    /// # Arguments
    ///
    /// * `location` identifies what weather history to get.
    /// * `date_range` controls the weather history dates.
    ///
    fn execute(&self, location: &Location, date_range: &DateRange) -> Result<()>;
    /// Query if the request has finished or return an error if there is no active request. `Ok(true)`
    /// guarantees the request response is available.
    ///
    fn poll(&self) -> Result<bool>;
    /// Get the request result by blocking until it finishes.
    ///
    fn get(&self) -> Result<DailyHistories>;
}
