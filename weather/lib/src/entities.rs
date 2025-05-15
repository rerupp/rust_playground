//! Structures used by the weather data `API`s.

use chrono::{NaiveDate, NaiveDateTime};

/// Used by front-ends to identify locations.
#[derive(Debug)]
pub struct DataCriteria {
    /// The locations of interest.
    pub filters: Vec<String>,
    /// If `true` the location filters will ignore case.
    pub icase: bool,
    /// If `true` locations will be sorted by name.
    pub sort: bool,
}
impl DataCriteria {
    pub fn filters(mut self, filters: Vec<String>) -> Self {
        self.filters = filters;
        self
    }
}
impl Default for DataCriteria {
    fn default() -> Self {
        Self { filters: Default::default(), icase: true, sort: true }
    }
}

/// A locations daily weather history.
#[derive(Debug)]
pub struct DailyHistories {
    /// The location metadata.
    pub location: Location,
    /// The daily histories for a location.
    pub histories: Vec<History>,
}

/// A locations history dates.
#[derive(Debug)]
pub struct HistoryDates {
    /// The location metadata.
    pub location: Location,
    /// The history dates metadata.
    pub history_dates: Vec<DateRange>,
}

#[derive(Debug)]
/// A locations history summary.
pub struct HistorySummaries {
    pub location: Location,
    /// The number of weather data histories available.
    pub count: usize,
    /// The overall size of weather data in bytes (may or may not be available).
    pub overall_size: Option<usize>,
    /// The size in bytes of weather data.
    pub raw_size: Option<usize>,
    /// The size in bytes of weather data in the backing store.
    pub store_size: Option<usize>,
}

/// The data that comprises a location.
#[derive(Clone, Debug)]
pub struct Location {
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

/// The weather history data.
#[derive(Debug)]
pub struct History {
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

/// For a given `NaiveDate` return the next day `NaiveDate`.
macro_rules! next_day {
    ($nd:expr) => {
        // For the weather data use case this should always be okay
        $nd.succ_opt().unwrap()
    };
}

/// A locations weather data history dates.
#[derive(Debug)]
pub struct DateRanges {
    /// The location id.
    pub location_id: String,
    /// The location weather history dates, grouped as consecutive date ranges.
    pub date_ranges: Vec<DateRange>,
}
impl DateRanges {
    pub fn covers(&self, date: &NaiveDate) -> bool {
        self.date_ranges.iter().any(|date_range| date_range.covers(date))
    }
}

/// A container for a range of dates.
#[derive(Debug)]
pub struct DateRange {
    /// The starting date of the range.
    pub start: NaiveDate,
    /// The inclusive end date of the range.
    pub end: NaiveDate,
}
impl DateRange {
    /// Create a new instance of the date range.
    ///
    /// # Arguments
    ///
    /// * `from` is the starting date.
    /// * `thru` is the inclusive end date.
    pub fn new(start: NaiveDate, end: NaiveDate) -> DateRange {
        DateRange { start, end }
    }
    /// Returns `true` if the *from* and *to* dates are equal.
    pub fn is_one_day(&self) -> bool {
        &self.start == &self.end
    }
    /// Identifies if a date is within the date range.
    ///
    /// # Arguments
    ///
    /// * `date` is the date that will be checked.
    pub fn covers(&self, date: &NaiveDate) -> bool {
        date >= &self.start && date <= &self.end
    }
    /// Allow the history range to be iterated over without consuming it.
    pub fn iter(&self) -> DateRangeIterator {
        DateRangeIterator { from: self.start, thru: self.end }
    }
    /// Returns the dates as a tuple of ISO8601 formatted strings.
    pub fn as_iso8601(&self) -> (String, String) {
        use toolslib::date_time::isodate;
        (isodate(&self.start), isodate(&self.end))
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
    pub fn from_dates(mut dates: Vec<NaiveDate>) -> Vec<DateRange> {
        // let mut dates = dates.clone();
        // dates.sort_by(|lhs, rhs| lhs.cmp(rhs));
        dates.sort_unstable();
        let mut history_ranges = vec![];
        let dates_len = dates.len();
        if dates_len == 1 {
            history_ranges.push(DateRange::new(dates[0], dates[0]));
        } else if dates_len > 1 {
            let mut from = dates[0];
            let mut to = dates[0];
            for i in 1..dates_len {
                if next_day!(to) != dates[i] {
                    history_ranges.push(DateRange::new(from, to));
                    from = dates[i];
                    to = dates[i];
                } else {
                    to = dates[i];
                }
            }
            history_ranges.push(DateRange::new(from, to));
        }
        history_ranges
    }
}
/// Create an iterator that will return all dates within the range.
impl IntoIterator for DateRange {
    type Item = NaiveDate;
    type IntoIter = DateRangeIterator;
    fn into_iter(self) -> Self::IntoIter {
        DateRangeIterator { from: self.start, thru: self.end }
    }
}
/// Create an iterator that will return all dates within the range.
impl IntoIterator for &DateRange {
    type Item = NaiveDate;
    type IntoIter = DateRangeIterator;
    fn into_iter(self) -> Self::IntoIter {
        DateRangeIterator { from: self.start, thru: self.end }
    }
}

/// Create the DateRange iterator structure.
#[derive(Debug)]
///
/// # Arguments
///
/// * `from` is the starting date.
/// * `thru` is the inclusive end date.
pub struct DateRangeIterator {
    /// The starting date.
    from: NaiveDate,
    /// The inclusive end date.
    thru: NaiveDate,
}
/// The implementation of iterating over the date range.
impl Iterator for DateRangeIterator {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.from > self.thru {
            None
        } else {
            let date = self.from;
            self.from = next_day!(date);
            Some(date)
        }
    }
}

/// The search criteria for locations.
#[derive(Debug)]
pub struct LocationCriteria {
    /// The optional city name.
    pub name: Option<String>,
    /// The optional state name.
    pub state: Option<String>,
    /// Used to limit the result of the query.
    pub limit: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolslib::date_time::get_date;

    #[test]
    pub fn iterate() {
        let range = DateRange::new(get_date(2022, 6, 1), get_date(2022, 6, 30));
        let mut testcase = range.start.clone();
        let test_cases: Vec<NaiveDate> = range.into_iter().collect();
        assert_eq!(test_cases.len(), 30);
        for day in 0..30 {
            assert_eq!(test_cases[day], testcase);
            // test_case = test_case.succ();
            testcase = next_day!(testcase);
        }
    }

    #[test]
    pub fn empty_history_range() {
        assert!(DateRange::from_dates(vec![]).is_empty());
    }

    #[test]
    pub fn single_history_range() {
        let test_date = get_date(2022, 7, 6);
        let testcase = DateRange::from_dates(vec![test_date]);
        assert_eq!(testcase.len(), 1);
        assert!(testcase[0].is_one_day());
        assert_eq!(test_date, testcase[0].start);
        assert_eq!(test_date, testcase[0].end);
        let (from, to) = testcase[0].as_iso8601();
        assert_eq!(from, to);
    }

    #[test]
    fn is_within() {
        let testcase = DateRange::new(get_date(2023, 7, 1), get_date(2023, 7, 31));
        assert!(testcase.covers(&get_date(2023, 7, 1)));
        assert!(!testcase.covers(&get_date(2023, 6, 30)));
        assert!(testcase.covers(&get_date(2023, 7, 31)));
        assert!(!testcase.covers(&get_date(2023, 8, 1)));
    }

    #[test]
    pub fn multiple_history_range() {
        let test_dates = vec![get_date(2022, 7, 3), get_date(2022, 6, 30), get_date(2022, 7, 4), get_date(2022, 7, 1)];
        let test_case = DateRange::from_dates(test_dates.clone());
        assert_eq!(test_case.len(), 2);
        assert_eq!(test_dates[1], test_case[0].start);
        assert_eq!(test_dates[3], test_case[0].end);
        assert_eq!(test_dates[0], test_case[1].start);
        assert_eq!(test_dates[2], test_case[1].end);
    }

    #[test]
    pub fn to_iso8601_history_range() {
        let test_case = DateRange::new(get_date(2022, 7, 1), get_date(2022, 7, 2));
        let (from, to) = test_case.as_iso8601();
        assert_eq!(from, "2022-07-01");
        assert_eq!(to, "2022-07-02");
    }
}
