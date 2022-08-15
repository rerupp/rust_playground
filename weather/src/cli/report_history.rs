//! # The implementation for report history (`rh`).
//!
//! The report history command presents historical weather data details.
//! The details shown depend on what command line flags are supplied. By default
//! the command will show the high and low temperatures for a date.
//!
//! Currently only 1 location can reported at a time however the command does
//! support case-insensitive searching.
//!
use chrono::prelude::*;
use clap::Args;
use crate::cli::ReportWriter;

use crate::domain::{DailyHistoryQuery, HistoryRange, LocationDailyHistories, LocationQuery, WeatherData};

// use super::{CliError, CliResult, ReportGenerator, util};
use super::{CliError, CliResult, ReportGenerator};

/// The CLI flags and arguments specific to the report history subcommand.
#[derive(Args, Debug)]
pub struct CommandArgs {
    /// Include daily temperatures in the report (default).
    ///
    /// The report will include the following:
    ///
    /// * history date
    /// * high temperature and time of day
    /// * low temperature and time of day
    #[clap(short, long, value_parser, conflicts_with = "all")]
    temp: bool,
    /// Include daily conditions in the report.
    ///
    /// The report will include the following:
    ///
    /// * history date
    /// * wind speed, wind bearing, maximum wind gust, and wind gust time of day
    /// * percentage of cloud cover
    /// * UV index and time of day
    #[clap(short, long, value_parser, conflicts_with = "all")]
    cnd: bool,
    /// Include min/max temperatures in the report.
    ///
    /// The report will include the following:
    ///
    /// * history date
    /// * maximum temperature and time of day
    /// * minimum temperature and time of day
    #[clap(short, long, value_parser, conflicts_with = "all")]
    max: bool,
    /// Include a summary of the weather in the report.
    ///
    /// The report will include the following:
    ///
    /// * history date
    /// * sunrise and sunset time of day
    /// * moon phase
    /// * max humidity
    /// * dew point temperature
    /// * a summary of the daily weather
    #[clap(short, long, value_parser, conflicts_with = "all")]
    sum: bool,
    /// Include all data in the generated report.
    ///
    /// This is equivalent of supplying the flags `-tcms` on the command line.
    #[clap(short, long, value_parser)]
    all: bool,
    /// The location used for the details report.
    #[clap(forbid_empty_values = true, validator = validate_location)]
    location: String,
    /// The starting date for the report.
    ///
    /// The form of the date can be YYYY-MM-DD, MM-DD-YYYY, or MMM-DD-YYYY
    /// where MMM is Jan, Feb, etc.
    #[clap(forbid_empty_values = true, validator = validate_date_string)]
    start: String,
    /// The ending date for the report
    ///
    /// The form of the date can be YYYY-MM-DD, MM-DD-YYYY, or MMM-DD-YYYY
    /// where MMM is Jan, Feb, etc. If the argument is not given history will
    /// be generated for the start date only.
    #[clap(forbid_empty_values = true, validator = validate_date_string)]
    ends: Option<String>,
}

/// The implementation for report history command flags.
impl CommandArgs {

    /// Returns true if the `temp` flag is supplied, `all` has been selected, or no report flags
    /// supplied.
    fn is_temp(&self) -> bool { self.temp || self.all || !(self.max || self.cnd || self.sum) }
    /// Returns true if the `max` flag or `all` flag has been supplied.
    fn is_max(&self) -> bool { self.max || self.all }
    /// Returns true if the `cnd` flag or `all` flag has been supplied.
    fn is_cnd(&self) -> bool { self.cnd || self.all }
    /// Returns true if the `sum` flag or `all` flag has been supplied.
    fn is_sum(&self) -> bool { self.sum || self.all }
}

/// The contents of the report history command.
pub struct ReportHistory {
    /// The command arguments.
    args: CommandArgs,
}

impl ReportHistory {
    /// Create a new instance of the report history command.
    ///
    /// # Arguments
    ///
    /// * `args` - the command arguments association with the instance.
    ///
    pub fn new(args: CommandArgs) -> ReportHistory {
        ReportHistory { args }
    }

    /// Returns the daily history details used to generate reports.
    ///
    /// # Arguments
    ///
    /// `weather_data` - the `domain` instance that will be used.
    ///
    fn get_daily_histories(&self, weather_data: &WeatherData) -> CliResult<LocationDailyHistories> {
        let lower = parse_date(&self.args.start)?;
        let upper = if let Some(ends) = &self.args.ends {
            parse_date(&ends)?
        } else {
            lower.clone()
        };
        let query = LocationQuery {
            location_filter: vec![self.args.location.clone()],
            sort: false,
            case_insensitive: true,
        };
        let history_query = DailyHistoryQuery {
            history_range: HistoryRange::new(lower, upper),
        };
        // println!("{:?} {:?}" query, history_query);
        weather_data.get_daily_history(query, history_query)
    }
}

/// The implementation of the `ReportGenerator` trait for list history.
impl ReportGenerator for ReportHistory {
    /// Generates a text based report for report history.
    ///
    /// An error will be returned if there are issues getting the location history details from
    /// the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn text_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
        text::output(self.get_daily_histories(weather_data)?, &self.args, report_writer)
    }

    /// Generates a JSON report for report history.
    ///
    /// An error will be returned if there are issues getting the location history details from
    /// the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    fn json_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> CliResult<()> {
        let location_daily_histories = self.get_daily_histories(weather_data)?;
        json::output(location_daily_histories, &self.args, report_writer, pretty)
    }

    /// Generates a CSV report for report history.
    ///
    /// An error will be returned if there are issues getting the location history details from
    /// the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn csv_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
            csv::output(self.get_daily_histories(weather_data)?, &self.args, report_writer)
    }
}

/// Used by the parser to validate the location name.
///
/// This will protect the location name containing either leading space, trailing space or not being
/// provided.
///
/// # Arguments
///
/// * `location` - the location string provided on the command line.
fn validate_location(location: &str) -> Result<(), String> {
    if location.trim().len() != location.len() {
        Err("The location name cannot have leading or trailing spaces".to_string())
    } else {
        // just in case someone forgot to give a location, check to see if it's a date
        match parse_date(location) {
            Err(_) => Ok(()),
            _ => Err(format!("The location appears to be a start date..."))
        }
    }
}

/// Used by the parser to validate the date strings that were entered.
///
/// Check the [parse date](parse_date) function to see what date string are acceptable.
///
/// # Arguments
///
/// * `date_str` - the date string that will be validated.
///
/// An error will be returned if there are errors validating the date.
fn validate_date_string(date_str: &str) -> Result<(), String> {
    match parse_date(&date_str) {
        Ok(_) => Ok(()),
        Err(error) => Err(error.to_string())
    }
}

/// Converts a date string to a UTC date.
///
/// The date can have the following forms:
///
/// * `YYYY-MM-DD` - where YYYY is the 4 digit year, MM is the 2 digit month, and DD the 2 digit
/// day of month.
/// * `MM-DD-YYYY` - where MM is the 2 digit month, DD is the 2 digit day of month, and YYYY is the
/// 4 digit year.
/// * `MMM-DD-YYYY` - where MMM is the abbreviated month name (always 3 characters), DD is the 2
/// digit day of month, and YYYY is the 4 digit year.
///
/// # Arguments
///
/// * `date_str` - the date string that will be validated.
///
/// An error will be returned if the date parsing fails.
fn parse_date(date_str: &str) -> CliResult<Date<Utc>> {
    for fmt in ["%Y-%m-%d", "%m-%d-%Y", "%b-%d-%Y"] {
        if let Ok(naive_date) = NaiveDate::parse_from_str(date_str, fmt) {
            return Ok(Date::<Utc>::from_utc(naive_date, Utc));
        }
    }
    Err(CliError::new(
        &format!("'{}' pattern must be 'YYYY-MM-DD', 'MM-DD-YYYY', or 'MMM-DD-YYYY'!!!", date_str)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_flags() {
        let mk_testcase = |temp, max, cnd, sum, all| -> CommandArgs {
            CommandArgs {
                temp,
                max,
                cnd,
                sum,
                all,
                location: "".to_string(),
                start: "".to_string(),
                ends: None,
            }
        };
        let test_case = mk_testcase(false, false, false, false, false);
        assert!(test_case.is_temp());
        assert!(!test_case.is_max());
        assert!(!test_case.is_cnd());
        assert!(!test_case.is_sum());
        let test_case = mk_testcase(true, false, false, false, false);
        assert!(test_case.is_temp());
        assert!(!test_case.is_max());
        assert!(!test_case.is_cnd());
        assert!(!test_case.is_sum());
        let test_case = mk_testcase(false, true, false, false, false);
        assert!(!test_case.is_temp());
        assert!(test_case.is_max());
        assert!(!test_case.is_cnd());
        assert!(!test_case.is_sum());
        let test_case = mk_testcase(false, false, true, false, false);
        assert!(!test_case.is_temp());
        assert!(!test_case.is_max());
        assert!(test_case.is_cnd());
        assert!(!test_case.is_sum());
        let test_case = mk_testcase(false, false, false, true, false);
        assert!(!test_case.is_temp());
        assert!(!test_case.is_max());
        assert!(!test_case.is_cnd());
        assert!(test_case.is_sum());
        let test_case = mk_testcase(false, false, false, false, true);
        assert!(test_case.is_temp());
        assert!(test_case.is_max());
        assert!(test_case.is_cnd());
        assert!(test_case.is_sum());
    }

    #[test]
    fn validate_dates() {
        assert!(validate_date_string("2022-7-15").is_ok());
        assert!(validate_date_string("7-1-2022").is_ok());
        assert!(validate_date_string("7-1-22").is_ok());
        assert!(validate_date_string("jul-15-2022").is_ok());
        assert!(validate_date_string("Jul-15-2022").is_ok());
        assert!(validate_date_string("JUL-15-2022").is_ok());
        assert!(validate_date_string("JUL-15-22").is_ok());
        assert!(validate_date_string("JULY-15-22").is_err());
    }

    #[test]
    fn parse_dates() {
        assert_eq!(parse_date("2022-7-15").unwrap(), Utc.ymd(2022, 7, 15));
        assert_eq!(parse_date("7-1-2022").unwrap(), Utc.ymd(2022, 7, 1));
        assert_eq!(parse_date("7-1-22").unwrap(), Utc.ymd(7, 1, 22));
        assert_eq!(parse_date("1-7-22").unwrap(), Utc.ymd(1, 7, 22));
        assert_eq!(parse_date("jul-15-2022").unwrap(), Utc.ymd(2022, 7, 15));
        assert_eq!(parse_date("Jul-15-2022").unwrap(), Utc.ymd(2022, 7, 15));
        assert_eq!(parse_date("JUL-15-2022").unwrap(), Utc.ymd(2022, 7, 15));
        assert_eq!(parse_date("JUL-15-22").unwrap(), Utc.ymd(22, 7, 15));
        assert!(parse_date("JULY-15-22").is_err());
    }

    #[test]
    fn validate_locations() {
        assert!(validate_location(" name").is_err());
        assert!(validate_location("name ").is_err());
        assert!(validate_location("name").is_ok());
    }
}

/// The report history text based reporting implementation.
///
/// This module utilizes the `text_reports` module to generate reports.
///
mod text {
    use chrono_tz::*;

    use crate::cli::text_reports::{Alignment, ColumnContent, ColumnProperty, fmt_float, fmt_yyyymmdd, ReportBuilder, ReportColumns};

    use super::*;

    /// Generates the report history text based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_daily_histories` - The location and history details that will be reported.
    /// * `report_args` - The report arguments to allow selection of the report details.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(location_daily_histories: LocationDailyHistories,
                  report_args: &CommandArgs,
                  report_writer: &ReportWriter) -> CliResult<()>
    {
        let mut report_columns = ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Center),
        ]);
        if report_args.is_temp() {
            report_columns.add_columns(vec![
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
            ]);
        }
        if report_args.is_max() {
            report_columns.add_columns(vec![
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
            ]);
        }
        if report_args.is_cnd() {
            report_columns.add_columns(vec![
                ColumnProperty::new(Alignment::Right),
                ColumnProperty::new(Alignment::Right),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
            ]);
        }
        if report_args.is_sum() {
            report_columns.add_columns(vec![
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Center),
                ColumnProperty::new(Alignment::Left),
            ]);
        }

        let mut report_builder = ReportBuilder::new(report_columns);
        let mut header_builder = ReportBuilder::header();
        header_builder.add_content(ColumnContent::new(""));
        if report_args.is_temp() {
            header_builder.add_contents(vec![
                ColumnContent::new("High"),
                ColumnContent::new("High"),
                ColumnContent::new("Low"),
                ColumnContent::new("Low"),
            ]);
        }
        if report_args.is_max() {
            header_builder.add_contents(vec![
                ColumnContent::new("Maximum"),
                ColumnContent::new("Maximum"),
                ColumnContent::new("Minimum"),
                ColumnContent::new("Minimum"),
            ]);
        }
        if report_args.is_cnd() {
            header_builder.add_contents(vec![
                ColumnContent::new("Wind"),
                ColumnContent::new("Wind"),
                ColumnContent::new("Wind"),
                ColumnContent::new("Wind"),
                ColumnContent::new("Cloud"),
                ColumnContent::new("UV"),
                ColumnContent::new("UV"),
            ]);
        }
        if report_args.is_sum() {
            header_builder.add_contents(vec![
                ColumnContent::new(""),
                ColumnContent::new(""),
                ColumnContent::new("Moon"),
                ColumnContent::new(""),
                ColumnContent::new("Dew"),
                ColumnContent::new("Summary"),
            ]);
        }
        report_builder.add_contents(header_builder)?;

        let mut header_builder = ReportBuilder::header();
        header_builder.add_content(ColumnContent::new("Date"));
        if report_args.is_temp() {
            header_builder.add_contents(vec![
                ColumnContent::new("Temperature"),
                ColumnContent::new("Temperature TOD"),
                ColumnContent::new("Temperature"),
                ColumnContent::new("Temperature TOD"),
            ]);
        }
        if report_args.is_max() {
            header_builder.add_contents(vec![
                ColumnContent::new("Temperature"),
                ColumnContent::new("Temperature TOD"),
                ColumnContent::new("Temperature"),
                ColumnContent::new("Temperature TOD"),
            ]);
        }
        if report_args.is_cnd() {
            header_builder.add_contents(vec![
                ColumnContent::new("Speed"),
                ColumnContent::new("Gust"),
                ColumnContent::new("Gust TOD"),
                ColumnContent::new("Bearing"),
                ColumnContent::new("Cover"),
                ColumnContent::new("Index"),
                ColumnContent::new("Index TOD"),
            ]);
        }
        if report_args.is_sum() {
            header_builder.add_contents(vec![
                ColumnContent::new("Sunrise"),
                ColumnContent::new("Sunset"),
                ColumnContent::new("Phase"),
                ColumnContent::new("Humidity"),
                ColumnContent::new("Point"),
                ColumnContent::new("Summary"),
            ]);
        }
        report_builder.add_contents(header_builder)?;
        report_builder.add_separator('-');
        if let Some((location, daily_histories)) = location_daily_histories {
            let tz: Tz = location.tz.parse().unwrap();
            for daily_history in daily_histories.daily_histories {
                let mut detail_builder = ReportBuilder::detail();
                detail_builder.add_content(ColumnContent::new(&fmt_yyyymmdd(&daily_history.date)));
                if report_args.is_temp() {
                    detail_builder.add_contents(vec![
                        ColumnContent::new(&fmt_temperature(&daily_history.temperature_high)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.temperature_high_time, &tz)),
                        ColumnContent::new(&fmt_temperature(&daily_history.temperature_low)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.temperature_low_time, &tz)),
                    ]);
                }
                if report_args.is_max() {
                    detail_builder.add_contents(vec![
                        ColumnContent::new(&fmt_temperature(&daily_history.temperature_max)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.temperature_max_time, &tz)),
                        ColumnContent::new(&fmt_temperature(&daily_history.temperature_min)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.temperature_min_time, &tz)),
                    ]);
                }
                if report_args.is_cnd() {
                    detail_builder.add_contents(vec![
                        ColumnContent::new(&fmt_float(&daily_history.wind_speed, 1)),
                        ColumnContent::new(&fmt_float(&daily_history.wind_gust, 1)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.wind_gust_time, &tz)),
                        ColumnContent::new(fmt_wind_bearing(&daily_history.wind_bearing)),
                        ColumnContent::new(&fmt_percent(&daily_history.cloud_cover)),
                        ColumnContent::new(fmt_uv_index(&daily_history.uv_index)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.uv_index_time, &tz)),
                    ]);
                }
                if report_args.is_sum() {
                    detail_builder.add_contents(vec![
                        ColumnContent::new(&fmt_hhmm(&daily_history.sunrise_time, &tz)),
                        ColumnContent::new(&fmt_hhmm(&daily_history.sunset_time, &tz)),
                        ColumnContent::new(&fmt_moon_phase(&daily_history.moon_phase)),
                        ColumnContent::new(&fmt_percent(&daily_history.humidity)),
                        ColumnContent::new(&fmt_temperature(&daily_history.dew_point)),
                        ColumnContent::new(if let Some(summary) = &daily_history.summary {
                            &summary
                        } else {
                            ""
                        }),
                    ]);
                }
                report_builder.add_contents(detail_builder)?;
            }
        }
        report_builder.output(report_writer)
    }

    /// Returns a compass bearing as a human readable direction.
    ///
    /// The four cardinal points on a compass are subdivided into a finer grained
    /// direction strings as shown below:
    ///
    /// ```
    /// N NNE NE ENE
    /// E ESE SE SSE
    /// S SSW SW WSW
    /// W WNW NW NNW
    /// ```
    ///
    /// There is a window around the absolute direction to determine the bearing string.
    /// As an example any bearing between 348.75 degrees and 11.25 degrees will be returned
    /// as a `N` bearing string.
    ///
    /// If the option is `None` an empty string will be returned.
    ///
    /// # Arguments
    ///
    /// * `bearing_option` - the bearing that will be converter to a string.
    ///
    pub fn fmt_wind_bearing(bearing_option: &Option<i64>) -> &'static str {
        if let Some(bearing) = bearing_option {
            static BEARINGS: [&'static str; 16] = [
                "N", "NNE", "NE", "ENE",
                "E", "ESE", "SE", "SSE",
                "S", "SSW", "SW", "WSW",
                "W", "WNW", "NW", "NNW"
            ];
            let index = ((*bearing as f64 / 22.5) + 0.5) as usize % 16;
            BEARINGS[index]
        } else {
            ""
        }
    }

    /// Returns a percentage as a string.
    ///
    /// The percentage is rounded to an integer value and contains a *%* trailing the value.
    /// The following table provides sample output.
    ///
    /// | Value | Result |
    /// | ---: | ---: |
    /// | 0.0 | 0% |
    /// | 25.4 | 25% |
    /// | 99.5 | 100% |
    ///
    /// If the option is `None` an empty string will be returned.
    ///
    fn fmt_percent(option: &Option<f64>) -> String {
        match option {
            Some(value) => format!("{:>3}%", ((value * 100.0) + 0.5) as i64),
            None => "".to_string(),
        }
    }

    /// Returns a temperature as a string.
    ///
    /// The temperature is rounded to the nearest 1/10 degree.
    ///
    /// If the option is `None` an empty string will be returned.
    ///
    #[inline]
    fn fmt_temperature(option: &Option<f64>) -> String {
        match option {
            Some(temperature) => format!("{:>-5.1}", temperature),
            None => "".to_string(),
        }
    }

    /// Returns a timestamp as hours and minutes string.
    ///
    /// The string will follow the form `hh:mm` where:
    ///
    /// * `hh` is the 2 digit hour (0-23)
    /// * `mm` is the hour minutes (0-59)
    ///
    /// If the option is `None` an empty string will be returned.
    ///
    #[inline]
    fn fmt_hhmm(option: &Option<i64>, tz: &Tz) -> String {
        match option {
            Some(datetime) => format!("{}", tz.timestamp(*datetime, 0).format("%H:%M")),
            None => "".to_string(),
        }
    }

    /// Returns a UV index as a human readable string.
    ///
    /// The possible UV index strings are:
    ///
    /// | UV Index | Description |
    /// | :----: | :----: |
    /// | 1-2 | low |
    /// | 3-5 | moderate |
    /// | 6-7 | high |
    /// | 8-10 | very high |
    /// | 11+ | extreme |
    ///
    /// If the option is `None` or the value 0, an empty string will be returned.
    ///
    fn fmt_uv_index(option: &Option<i64>) -> &'static str {
        let mut uv_index = "";
        if let Some(value) = option {
            if *value > 0 {
                uv_index = match value {
                    1 | 2 => "low",
                    3 | 4 | 5 => "moderate",
                    6 | 7 => "high",
                    8 | 9 | 10 => "very high",
                    _ => "extreme"
                };
            }
        }
        uv_index
    }

    /// Returns moon phase as a human readable string.
    ///
    /// The possible moon phase indicators are:
    ///
    /// | Moon Phase | Description |
    /// | :----: | :----: |
    /// | 0 | new moon |
    /// | 0-0.25 | waxing crescent |
    /// | 0.25 | first quarter |
    /// | 0.25-0.5 | waxing gibbous |
    /// | 0.5 | full moon |
    /// | 0.5-0.75 | waning gibbous |
    /// | 0.75 | last quarter |
    /// | 0.75-1.0 | waning crescent |
    ///
    /// If the option is `None` an empty string will be returned.
    ///
    fn fmt_moon_phase(option: &Option<f64>) -> &'static str {
        let mut moon_phase = "";
        if let Some(value) = option {
            let phase = *value;
            moon_phase = if phase >= 0.0 && phase <= 0.01 {
                "new moon"
            } else if phase > 0.01 && phase < 0.24 {
                "waxing crescent"
            } else if phase >= 0.24 && phase <= 0.26 {
                "first quarter"
            } else if phase > 0.26 && phase < 0.49 {
                "waxing gibbous"
            } else if phase >= 0.49 && phase <= 0.51 {
                "full moon"
            } else if phase > 0.51 && phase < 0.74 {
                "waning gibbous"
            } else if phase >= 0.74 && phase <= 0.76 {
                "last quarter"
            } else if phase > 0.76 && phase <= 1.0 {
                "waning crescent"
            } else {
                "unknown"
            };
        }
        moon_phase
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn hhmm() {
            let tz: Tz = "America/Phoenix".parse().unwrap();
            assert_eq!(fmt_hhmm(&None, &tz), "");
            assert_eq!(fmt_hhmm(&Some(1588371720), &tz), "15:22");
        }

        #[test]
        fn wind_bearing() {
            for bearing in 0..=11 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "N"); }
            for bearing in 12..=33 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "NNE"); }
            for bearing in 34..=56 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "NE"); }
            for bearing in 57..=78 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "ENE"); }
            for bearing in 79..=101 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "E"); }
            for bearing in 102..=123 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "ESE"); }
            for bearing in 124..=146 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "SE"); }
            for bearing in 147..=168 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "SSE"); }
            for bearing in 169..=191 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "S"); }
            for bearing in 192..=213 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "SSW"); }
            for bearing in 214..=236 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "SW"); }
            for bearing in 237..=258 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "WSW"); }
            for bearing in 259..=281 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "W"); }
            for bearing in 282..=303 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "WNW"); }
            for bearing in 304..=326 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "NW"); }
            for bearing in 327..=348 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "NNW"); }
            for bearing in 349..=361 { assert_eq!(fmt_wind_bearing(&Some(bearing)), "N"); }
        }

        #[test]
        fn percent() {
            assert_eq!(fmt_percent(&None), "");
            assert_eq!(fmt_percent(&Some(0.0)), "  0%");
            assert_eq!(fmt_percent(&Some(0.1049)), " 10%");
            assert_eq!(fmt_percent(&Some(0.995)), "100%");
        }

        #[test]
        fn temperature() {
            assert_eq!(fmt_temperature(&None), "");
            assert_eq!(fmt_temperature(&Some(50.94)), " 50.9");
            assert_eq!(fmt_temperature(&Some(50.95)), " 51.0");
            assert_eq!(fmt_temperature(&Some(99.9)), " 99.9");
            assert_eq!(fmt_temperature(&Some(-29.9)), "-29.9");
        }

        #[test]
        fn uv_index() {
            assert_eq!(fmt_uv_index(&None), "");
            assert_eq!(fmt_uv_index(&Some(0)), "");
            assert_eq!(fmt_uv_index(&Some(1)), "low");
            assert_eq!(fmt_uv_index(&Some(2)), "low");
            assert_eq!(fmt_uv_index(&Some(3)), "moderate");
            assert_eq!(fmt_uv_index(&Some(4)), "moderate");
            assert_eq!(fmt_uv_index(&Some(5)), "moderate");
            assert_eq!(fmt_uv_index(&Some(6)), "high");
            assert_eq!(fmt_uv_index(&Some(7)), "high");
            assert_eq!(fmt_uv_index(&Some(8)), "very high");
            assert_eq!(fmt_uv_index(&Some(9)), "very high");
            assert_eq!(fmt_uv_index(&Some(10)), "very high");
            assert_eq!(fmt_uv_index(&Some(11)), "extreme");
            assert_eq!(fmt_uv_index(&Some(12)), "extreme");
        }

        #[test]
        fn moon_phase() {
            assert_eq!(fmt_moon_phase(&None), "");
            assert_eq!(fmt_moon_phase(&Some(0.0)), "new moon");
            assert_eq!(fmt_moon_phase(&Some(0.01)), "new moon");
            assert_eq!(fmt_moon_phase(&Some(0.011)), "waxing crescent");
            assert_eq!(fmt_moon_phase(&Some(0.239)), "waxing crescent");
            assert_eq!(fmt_moon_phase(&Some(0.24)), "first quarter");
            assert_eq!(fmt_moon_phase(&Some(0.26)), "first quarter");
            assert_eq!(fmt_moon_phase(&Some(0.261)), "waxing gibbous");
            assert_eq!(fmt_moon_phase(&Some(0.489)), "waxing gibbous");
            assert_eq!(fmt_moon_phase(&Some(0.49)), "full moon");
            assert_eq!(fmt_moon_phase(&Some(0.51)), "full moon");
            assert_eq!(fmt_moon_phase(&Some(0.511)), "waning gibbous");
            assert_eq!(fmt_moon_phase(&Some(0.739)), "waning gibbous");
            assert_eq!(fmt_moon_phase(&Some(0.74)), "last quarter");
            assert_eq!(fmt_moon_phase(&Some(0.76)), "last quarter");
            assert_eq!(fmt_moon_phase(&Some(0.761)), "waning crescent");
            assert_eq!(fmt_moon_phase(&Some(1.0)), "waning crescent");
            assert_eq!(fmt_moon_phase(&Some(1.001)), "unknown");
        }
    }
}

/// The report history JSON based reporting implementation.
///
/// This module utilizes the `serde_json` dependency to generate reports.
///
mod json {
    use chrono_tz::*;
    use serde_json::{json, to_string, to_string_pretty, Value};
    use serde_json::map::Map;

    use crate::cli::text_reports::fmt_yyyymmdd;

    use super::*;

    /// Generates the report history JSON based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_daily_histories` - The location and history details that will be reported.
    /// * `report_args` - The report arguments to allow selection of the report details.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(location_daily_histories: LocationDailyHistories,
                  report_args: &CommandArgs,
                  report_writer: &ReportWriter,
                  pretty: bool) -> CliResult<()>
    {
        if let Some((location, daily_histories)) = location_daily_histories {
            let mut histories: Vec<Map<String, Value>> = vec![];
            let tz: Tz = location.tz.parse().unwrap();
            for daily_history in daily_histories.daily_histories {
                let mut history = Map::new();
                let mut add = |key: &str, value: Value| {
                    history.insert(key.to_string(), value)
                };
                add("date", json!(fmt_yyyymmdd(&daily_history.date)));
                if report_args.is_temp() {
                    add("temperatureHigh", float_value(&daily_history.temperature_high));
                    add("temperatureHighTime", datetime_value(&daily_history.temperature_high_time, &tz));
                    add("temperatureLow", float_value(&daily_history.temperature_low));
                    add("temperatureLowTime", datetime_value(&daily_history.temperature_low_time, &tz));
                }
                if report_args.is_max() {
                    add("temperatureMax", float_value(&daily_history.temperature_max));
                    add("temperatureMaxTime", datetime_value(&daily_history.temperature_max_time, &tz));
                    add("temperatureMin", float_value(&daily_history.temperature_min));
                    add("temperatureMinTime", datetime_value(&daily_history.temperature_min_time, &tz));
                }
                if report_args.is_cnd() {
                    add("windSpeed", float_value(&daily_history.wind_speed));
                    add("windGust", float_value(&daily_history.wind_gust));
                    add("windGustTime", datetime_value(&daily_history.wind_gust_time, &tz));
                    add("windBearing", int_value(&daily_history.wind_bearing));
                    add("cloudCover", float_value(&daily_history.cloud_cover));
                    add("uvIndex", int_value(&daily_history.uv_index));
                    add("uvIndexTime", datetime_value(&daily_history.uv_index_time, &tz));
                }
                if report_args.is_sum() {
                    add("sunrise", datetime_value(&daily_history.sunrise_time, &tz));
                    add("sunset", datetime_value(&daily_history.sunset_time, &tz));
                    add("moonPhase", float_value(&daily_history.moon_phase));
                    add("humidity", float_value(&daily_history.humidity));
                    add("dewPoint", float_value(&daily_history.dew_point));
                    add("summary", string_value(&daily_history.summary));
                }
                histories.push(history);
            }
            let root = json!({
                "location": location.name,
                "type": Value::String("daily_history".to_string()),
                "history": json![histories],
            });
            let as_text = if pretty { to_string_pretty } else { to_string };
            writeln!(report_writer.create()?, "{}", as_text(&root)?)?;
        }
        Ok(())
    }

    /// Returns a `Value::String(...) ` containing an IETF RFC3339 date timestamp.
    ///
    /// The binary timestamp is converted to a string following the form `YYYY-MM-DDThh:mm:ss+hh:mm`
    /// where:
    ///
    /// * `YYYY` is the 4 digit year
    /// * `MM` is the 2 digit month
    /// * `DD` is the 2 digit day of month
    /// * `hh` is the 2 digit hour of day
    /// * `mm` is the 2 digit minutes within hour
    /// * `ss` is the 2 digit seconds within minute
    /// * `+hh:mm` is the timezone offset. This could be replaced with `Z` however there are no
    /// timezones currently within the UTC zone.
    ///
    /// If option is `None` a `Value::Null` will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the timestamp used to create the IETF datetime value.
    /// * `tz` - the timezone associated with the timestamp.
    ///
    fn datetime_value(option: &Option<i64>, tz: &Tz) -> Value {
        match option {
            Some(timestamp) => {
                let dt: DateTime<Tz> = tz.timestamp(*timestamp, 0);
                let iso8601 = dt.to_rfc3339_opts(SecondsFormat::Secs, true);
                json!(iso8601)
            },
            None => Value::Null,
        }
    }

    /// Returns a `Value::String(...)` containing a string value.
    ///
    /// If option is `None` a `Value::Null` will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the string that will encoded as a value.
    ///
    #[inline]
    fn string_value(option: &Option<String>) -> Value {
        match option {
            Some(string) => json!(string),
            None => Value::Null
        }
    }

    /// Returns a `Value::Number(...)` containing the integer value.
    ///
    /// If option is `None` a `Value::Null` will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the integer that will encoded as a value.
    ///
    #[inline]
    fn int_value(option: &Option<i64>) -> Value {
        match option {
            Some(int) => json!(int),
            None => Value::Null,
        }
    }

    /// Returns a `Value::Number(...)` containing the float value.
    ///
    /// If option is `None` a `Value::Null` will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the float that will encoded as a value.
    ///
    #[inline]
    fn float_value(option: &Option<f64>) -> Value {
        match option {
            Some(float) => json!(float),
            None => Value::Null,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn datetime() {
            let tz: Tz = "America/Los_Angeles".parse().unwrap();
            assert_eq!(datetime_value(&None, &tz), Value::Null);
            assert_eq!(datetime_value(&Some(1588399200), &tz), "2020-05-01T23:00:00-07:00".to_string());
        }

        #[test]
        fn strings() {
            assert_eq!(string_value(&None), Value::Null);
            let testcase = "foobar".to_string();
            assert_eq!(string_value(&Some(testcase.clone())), json!(testcase));
        }

        #[test]
        fn numbers() {
            assert_eq!(float_value(&None), Value::Null);
            assert_eq!(float_value(&Some(123.456)), json!(123.456));
            assert_eq!(int_value(&None), Value::Null);
            assert_eq!(int_value(&Some(123456)), json!(123456));
        }
    }
}

/// The report history CSV based reporting implementation.
///
/// This module utilizes the `csv` dependency to generate reports.
///
mod csv {
    use chrono::prelude::*;
    use chrono_tz::*;
    use csv::Writer;
    use crate::cli::ReportWriter;

    use crate::cli::text_reports::fmt_yyyymmdd;

    use super::{CliResult, CommandArgs, DateTime, LocationDailyHistories, SecondsFormat};

    /// Generates the list history CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_daily_histories` - The location and history details that will be reported.
    /// * `report_args` - The report arguments to allow selection of the report details.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(location_daily_histories: LocationDailyHistories,
                  report_args: &CommandArgs,
                  report_writer: &ReportWriter) -> CliResult<()>
    {
        let mut writer = Writer::from_writer(report_writer.create()?);
        let mut labels: Vec<&str> = vec!["date"];
        if report_args.is_temp() {
            labels.push("temperatureHigh");
            labels.push("temperatureHighTime");
            labels.push("temperatureLow");
            labels.push("temperatureLowTime");
        }
        if report_args.is_max() {
            labels.push("temperatureMax");
            labels.push("temperatureMaxTime");
            labels.push("temperatureMin");
            labels.push("temperatureMinTime");
        }
        if report_args.is_cnd() {
            labels.push("windSpeed");
            labels.push("windGust");
            labels.push("windGustTime");
            labels.push("windBearing");
            labels.push("cloudCover");
            labels.push("uvIndex");
            labels.push("uvIndexTime");
        }
        if report_args.is_sum() {
            labels.push("sunrise");
            labels.push("sunset");
            labels.push("moonPhase");
            labels.push("humidity");
            labels.push("dewPoint");
            labels.push("summary");
        }
        writer.write_record(&labels)?;
        if let Some((location, daily_histories)) = location_daily_histories {
            let tz: Tz = location.tz.parse().unwrap();
            for daily_history in daily_histories.daily_histories {
                let mut history = vec![fmt_yyyymmdd(&daily_history.date)];
                if report_args.is_temp() {
                    history.push(float_value(&daily_history.temperature_high));
                    history.push(datetime_value(&daily_history.temperature_high_time, &tz));
                    history.push(float_value(&daily_history.temperature_low));
                    history.push(datetime_value(&daily_history.temperature_low_time, &tz));
                }
                if report_args.is_max() {
                    history.push(float_value(&daily_history.temperature_max));
                    history.push(datetime_value(&daily_history.temperature_max_time, &tz));
                    history.push(float_value(&daily_history.temperature_min));
                    history.push(datetime_value(&daily_history.temperature_min_time, &tz));
                }
                if report_args.is_cnd() {
                    history.push(float_value(&daily_history.wind_speed));
                    history.push(float_value(&daily_history.wind_gust));
                    history.push(datetime_value(&daily_history.wind_gust_time, &tz));
                    history.push(int_value(&daily_history.wind_bearing));
                    history.push(float_value(&daily_history.cloud_cover));
                    history.push(int_value(&daily_history.uv_index));
                    history.push(datetime_value(&daily_history.uv_index_time, &tz));
                }
                if report_args.is_sum() {
                    history.push(datetime_value(&daily_history.sunrise_time, &tz));
                    history.push(datetime_value(&daily_history.sunset_time, &tz));
                    history.push(float_value(&daily_history.moon_phase));
                    history.push(float_value(&daily_history.humidity));
                    history.push(float_value(&daily_history.dew_point));
                    history.push(string_value(&daily_history.summary));
                }
                writer.write_record(&history)?;
            }
        }
        Ok(())
    }

    /// Returns an IETF RFC3339 date timestamp string.
    ///
    /// The binary timestamp is converted to a string following the form `YYYY-MM-DDThh:mm:ss+hh:mm`
    /// where:
    ///
    /// * `YYYY` is the 4 digit year
    /// * `MM` is the 2 digit month
    /// * `DD` is the 2 digit day of month
    /// * `hh` is the 2 digit hour of day
    /// * `mm` is the 2 digit minutes within hour
    /// * `ss` is the 2 digit seconds within minute
    /// * `+hh:mm` is the timezone offset. This could be replaced with `Z` however there are no
    /// timezones currently within the UTC zone.
    ///
    /// If option is `None` an empty string will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the timestamp used to create the IETF datetime value.
    /// * `tz` - the timezone associated with the timestamp.
    ///
    fn datetime_value(option: &Option<i64>, tz: &Tz) -> String {
        match option {
            Some(timestamp) => {
                let dt: DateTime<Tz> = tz.timestamp(*timestamp, 0);
                dt.to_rfc3339_opts(SecondsFormat::Secs, true)
            },
            None => "".to_string(),
        }
    }

    /// Returns a copy of a string value.
    ///
    /// If option is `None` an empty string will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the string that will be copied.
    ///
    #[inline]
    fn string_value(option: &Option<String>) -> String {
        match option {
            Some(string) => string.clone(),
            None => "".to_string(),
        }
    }

    /// Returns an integer value as a string value.
    ///
    /// If option is `None` an empty string will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the integer that will be converted to a string.
    ///
    #[inline]
    fn int_value(option: &Option<i64>) -> String {
        match option {
            Some(int) => int.to_string(),
            None => "".to_string(),
        }
    }

    /// Returns a float value as a string value.
    ///
    /// If option is `None` an empty string will be returned.
    ///
    /// # Arguments
    ///
    /// * `option` - the float that will be converted to a string.
    ///
    #[inline]
    fn float_value(option: &Option<f64>) -> String {
        match option {
            Some(float) => float.to_string(),
            None => "".to_string(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn datetime() {
            let tz: Tz = "America/Los_Angeles".parse().unwrap();
            assert_eq!(datetime_value(&None, &tz), "".to_string());
            assert_eq!(datetime_value(&Some(1588389540), &tz), "2020-05-01T20:19:00-07:00".to_string());
        }

        #[test]
        fn strings() {
            assert_eq!(string_value(&None), "".to_string());
            let testcase = "foobar".to_string();
            assert_eq!(string_value(&Some(testcase.clone())), testcase);
        }

        #[test]
        fn numbers() {
            assert_eq!(float_value(&None), "".to_string());
            assert_eq!(float_value(&Some(123.456)), 123.456.to_string());
            assert_eq!(int_value(&None), "".to_string());
            assert_eq!(int_value(&Some(123456)), 123456.to_string());
        }
    }
}
