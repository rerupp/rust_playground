//! The objects used to pass data between front-end and back-end.

use chrono::prelude::*;

/// The data that comprises a location.
#[derive(Debug)]
pub struct Location {
    /// The unique id of a location.
    pub id: String,
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

/// Used by the front-end to query what location data should be returned.
pub struct LocationQuery {
    /// If `true` the location filter should ignore case.
    pub case_insensitive: bool,
    /// If `true` the result will be sorted by location name.
    pub sort: bool,
    /// The list of locations that should be returned.
    pub location_filter: Vec<String>,
}

/// A locations history summary.
#[derive(Debug)]
pub struct HistorySummary {
    /// The location id.
    pub location_id: String,
    /// The number of weather data histories available.
    pub count: usize,
    /// The overall size of weather data for a location in bytes (may or may not be available).
    pub overall_size: Option<usize>,
    /// The raw size of weather data for a location in bytes (may or may not be available).
    pub raw_size: Option<usize>,
    /// The compressed data size of weather data for a location in bytes (may or may not be available).
    pub compressed_size: Option<usize>,
}

/// A locations history dates.
#[derive(Debug)]
pub struct HistoryDates {
    /// The location id.
    pub location_id: String,
    /// The list of location history data.
    pub history_ranges: Vec<HistoryRange>,
}

/// A locations weather history details.
#[derive(Debug)]
pub struct DailyHistories {
    /// The location id.
    pub location_id: String,
    /// The list of daily weather data information.
    pub daily_histories: Vec<DailyHistory>,
}

/// Identifies what weather data dates should be returned.
pub struct DailyHistoryQuery {
    pub history_range: HistoryRange,
}

/// The daily weather data.
#[derive(Debug)]
pub struct DailyHistory {
    /// The location id.
    pub location_id: String,
    /// The date associated with the weather data.
    pub date: NaiveDate,
    /// The high temperature.
    pub temperature_high: Option<f64>,
    /// The high temperature time of day.
    pub temperature_high_time: Option<i64>,
    /// The low temperature.
    pub temperature_low: Option<f64>,
    /// The low temperature time of day.
    pub temperature_low_time: Option<i64>,
    /// The maximum temperature.
    pub temperature_max: Option<f64>,
    /// The maximum temperature time of day.
    pub temperature_max_time: Option<i64>,
    /// The minimum temperature.
    pub temperature_min: Option<f64>,
    /// The minimum temperature time of day.
    pub temperature_min_time: Option<i64>,
    /// The average wind speed.
    pub wind_speed: Option<f64>,
    /// The maximum wind speed.
    pub wind_gust: Option<f64>,
    /// The maximum wind speed time of day.
    pub wind_gust_time: Option<i64>,
    /// The predominate wind direction.
    pub wind_bearing: Option<i64>,
    /// The percent of cloud cover.
    pub cloud_cover: Option<f64>,
    /// The UV index.
    pub uv_index: Option<i64>,
    /// The UV time of day.
    pub uv_index_time: Option<i64>,
    /// A textual summary of the daily weather.
    pub summary: Option<String>,
    /// The average humidity.
    pub humidity: Option<f64>,
    /// The dew point.
    pub dew_point: Option<f64>,
    /// The sunrise time of day.
    pub sunrise_time: Option<i64>,
    /// The sunset time of day.
    pub sunset_time: Option<i64>,
    /// The phase of the moon.
    pub moon_phase: Option<f64>,
}

/// A container for a range of dates.
#[derive(Debug)]
pub struct HistoryRange {
    /// The starting date of the range.
    pub from: NaiveDate,
    /// The inclusive end date of the range.
    pub to: NaiveDate,
}

impl HistoryRange {
    pub fn new(from: NaiveDate, to: NaiveDate) -> HistoryRange {
        HistoryRange { from, to }
    }
    /// Returns `true` if the *from* and *to* dates are equal.
    pub fn is_one_day(&self) -> bool {
        &self.from == &self.to
    }
    /// Returns the dates as a tuple of ISO8601 formatted strings.
    pub fn as_iso8601(&self) -> (String, String) {
        (self.from.format("%F").to_string(), self.to.format("%F").to_string())
    }

    /// A helper that builds a list of history range from a list of dates.
    ///
    /// As an example, if the following date list was passed to the function:
    ///
    /// * 2022-08-12
    /// * 2022-08-10
    /// * 2022-08-14
    ///
    /// The resulting list of date ranges would be returned.
    ///
    /// * (2022-08-10, 2022-08-10)
    /// * (2022-08-12, 2022-08-14)
    ///
    /// # Arguments
    ///
    /// * `dates` - The list of dates that will be converted to date ranges.
    ///
    pub fn from_dates(dates: &Vec<NaiveDate>) -> Vec<HistoryRange> {
        let mut dates = dates.clone();
        dates.sort_by(|lhs, rhs| lhs.cmp(rhs));
        let mut history_ranges = vec![];
        let dates_len = dates.len();
        if dates_len == 1 {
            history_ranges.push(HistoryRange::new(dates[0], dates[0]));
        } else if dates_len > 1 {
            let mut from = dates[0];
            let mut to = dates[0];
            for i in 1..dates_len {
                if next_day(&to) != dates[i] {
                    history_ranges.push(HistoryRange::new(from, to));
                    from = dates[i];
                    to = dates[i];
                } else {
                    to = dates[i];
                }
            }
            history_ranges.push(HistoryRange::new(from, to));
        }
        history_ranges
    }
}

/// Create an iterator that will return all dates within the range.
impl IntoIterator for HistoryRange {
    type Item = NaiveDate;
    type IntoIter = DateRangeIterator;
    fn into_iter(self) -> Self::IntoIter {
        DateRangeIterator { from: self.from, to: self.to }
    }
}

/// Create the DateRange iterator structure.
pub struct DateRangeIterator {
    from: NaiveDate,
    to: NaiveDate,
}

/// The implementation of iterating over the date range.
impl Iterator for DateRangeIterator {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.from > self.to {
            None
        } else {
            let date = self.from;
            self.from = next_day(&date);
            Some(date)
        }
    }
}

// for thest use cases this should always be okay
#[inline]
fn next_day(nd: &NaiveDate) -> NaiveDate {
    nd.succ_opt().unwrap()
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use toolslib::date_time::get_date;

    #[test]
    pub fn iterate() {
        let range = HistoryRange::new(get_date(2022, 6, 1), get_date(2022, 6, 30));
        let mut testcase = range.from.clone();
        let test_cases: Vec<NaiveDate> = range.into_iter().collect();
        assert_eq!(test_cases.len(), 30);
        for day in 0..30 {
            assert_eq!(test_cases[day], testcase);
            // test_case = test_case.succ();
            testcase = next_day(&testcase);
        }
    }

    #[test]
    pub fn empty_history_range() {
        assert!(HistoryRange::from_dates(&vec![]).is_empty());
    }

    #[test]
    pub fn single_history_range() {
        let test_date = get_date(2022, 7, 6);
        let testcase = HistoryRange::from_dates(&vec![test_date]);
        assert_eq!(testcase.len(), 1);
        assert!(testcase[0].is_one_day());
        assert_eq!(test_date, testcase[0].from);
        assert_eq!(test_date, testcase[0].to);
        let (from, to) = testcase[0].as_iso8601();
        assert_eq!(from, to);
    }

    #[test]
    pub fn multiple_history_range() {
        let test_dates = vec![get_date(2022, 7, 3), get_date(2022, 6, 30), get_date(2022, 7, 4), get_date(2022, 7, 1)];
        let test_case = HistoryRange::from_dates(&test_dates);
        assert_eq!(test_case.len(), 2);
        assert_eq!(test_dates[1], test_case[0].from);
        assert_eq!(test_dates[3], test_case[0].to);
        assert_eq!(test_dates[0], test_case[1].from);
        assert_eq!(test_dates[2], test_case[1].to);
    }

    #[test]
    pub fn to_iso8601_history_range() {
        let test_case = HistoryRange::new(get_date(2022, 7, 1), get_date(2022, 7, 2));
        let (from, to) = test_case.as_iso8601();
        assert_eq!(from, "2022-07-01");
        assert_eq!(to, "2022-07-02");
    }
}
