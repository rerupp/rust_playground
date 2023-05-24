use super::{fmt, NaiveDate};

/// The parameters controlling what locations are of interest.
#[derive(Debug)]
pub struct LocationQuery {
    /// Identifies what locations are of interest.
    ///
    /// If filters are available they will be compared against the location name and alias.
    /// The filtering is a partial match. As an example, if you have a location named
    /// `Tigard, OR` a filter `Tig` will match however `OR` will not.
    ///
    pub filters: Vec<String>,
    /// Determines if the filter is case insensitive.
    ///
    /// As an example, if you have a location named `Las Vegas, NV` the filter `las` will match
    /// if `true` and will not if `false`.
    pub case_insensitive: bool,
    /// Determines if the returned locations should be sorted by their name.
    pub sort: bool,
}

/// The parameters controlling what location is used to return a summary of weather data.
#[derive(Debug)]
pub struct HistoryQuery {
    /// The location id of the weather data.
    pub location_id: String,
    /// The data returned should be sorted by date.
    pub sort: bool,
}

/// The parameters controlling what location weather data should be returned.
pub struct DailyHistoryQuery {
    /// The location id of the weather data.
    pub location_id: String,
    /// The range of weather data to return.
    pub history_bounds: HistoryBounds,
}

/// The history dates used when querying weather data.
pub struct HistoryBounds {
    /// The lower date boundary.
    pub lower: NaiveDate,
    /// The inclusive upper date boundary.
    pub upper: NaiveDate,
}

impl fmt::Display for HistoryBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.lower, self.upper)
    }
}

impl HistoryBounds {
    pub fn new(lower: NaiveDate, upper: NaiveDate) -> HistoryBounds {
        HistoryBounds { lower, upper }
    }
    /// Returns true if the date is within the lower and upper dates.
    ///
    /// # Arguments
    ///
    /// * `data` - the date that will be compared.
    pub fn contains(&self, date: &NaiveDate) -> bool {
        &self.lower <= date && date <= &self.upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;

    #[test]
    fn history_bounds() {
        fn mk_date(year: i32, month: u32, day: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(year, month, day).unwrap()
        }
        let history_bounds = HistoryBounds::new(mk_date(2022, 7, 1), mk_date(2022, 7, 31));
        assert_eq!(history_bounds.contains(&mk_date(2022, 6, 30)), false);
        assert!(history_bounds.contains(&mk_date(2022, 7, 1)));
        assert!(history_bounds.contains(&mk_date(2022, 7, 31)));
        assert_eq!(history_bounds.contains(&mk_date(2022, 8, 1)), false);
    }
}
