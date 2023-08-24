//! # The implementation for list history (`lh`).
//!
//! The list history command presents what historical weather data is available for a location.
//! The available data is shown as dates. If there are consecutive dates available they will be
//! shown as a range (ie. YYYY-MM-DD to YYYY-MM-DD).
//!
//! The command allows locations to be filtered. The filtering is case insensitive
//! and will match either the start of the location name or alias.
//!
use super::{ReportGenerator, ReportWriter, Result as CliResult};
use clap::Args;
use weather_lib::prelude::{DataCriteria, HistoryDates, WeatherData};

#[derive(Args, Debug)]
/// The command arguments for the list history command.
pub struct CommandArgs {
    /// Filter output to these locations (Optional).
    locations: Vec<String>,
}

/// The contents of the list history command.
pub struct ListHistory {
    /// The command arguments.
    args: CommandArgs,
}

impl ListHistory {
    /// Create a new instance of the list history command.
    ///
    /// # Arguments
    ///
    /// * `args` - the command arguments association with the instance.
    ///
    pub fn new(args: CommandArgs) -> ListHistory {
        ListHistory { args }
    }
    /// Get the history details used to generate reports.
    ///
    /// # Arguments
    ///
    /// `weather_data` - the `domain` instance that will be used.
    ///
    fn get_location_dates(&self, weather_data: &WeatherData) -> CliResult<Vec<HistoryDates>> {
        let criteria = DataCriteria { filters: self.args.locations.clone(), icase: true, sort: true };
        Ok(weather_data.get_history_dates(criteria)?)
    }
}

/// The implementation of the `ReportGenerator` trait for list history.
impl ReportGenerator for ListHistory {
    /// Generates a text based report for list history.
    ///
    /// An error will be returned if there are issues getting location history dates from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn text_output(&self, weather_api: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
        text::output(self.get_location_dates(weather_api)?, report_writer)
    }
    /// Generates a JSON report for list history.
    ///
    /// An error will be returned if there are issues getting location history dates from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    fn json_output(&self, weather_api: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> CliResult<()> {
        json::output(self.get_location_dates(weather_api)?, report_writer, pretty)
    }
    /// Generates a CSV report for list history.
    ///
    /// An error will be returned if there are issues getting location history dates from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn csv_output(&self, weather_api: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
        csv::output(self.get_location_dates(weather_api)?, report_writer)
    }
}

/// The list history text based reporting implementation.
///
/// This module utilizes the `text_reports` module to generate reports.
///
mod text {
    use super::*;
    use toolslib::{
        rptcols, rptrow,
        text::{write_strings, Report},
    };

    /// Generates the locations history text based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_history_dates` - The list of location and history dates that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(locations_history_dates: Vec<HistoryDates>, report_writer: &ReportWriter) -> CliResult<()> {
        let mut report = Report::from(rptcols!(<, <));
        report.header(rptrow!(^ "Location", ^ "History Dates")).separator("-");
        for location_history_dates in locations_history_dates {
            if location_history_dates.history_dates.is_empty() {
                report.text(rptrow!(location_history_dates.location.name));
            } else {
                let mut location_name = Some(location_history_dates.location.name);
                for history_range in location_history_dates.history_dates {
                    let (from, to) = history_range.as_iso8601();
                    let range = if history_range.is_one_day() { from } else { format!("{} to {}", from, to) };
                    let name = location_name.take().unwrap_or(String::default());
                    report.text(rptrow!(name, range));
                }
            }
        }
        write_strings(&mut report_writer.get_writer()?, report.into_iter())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}

/// The list history CSV based reporting implementation.
///
/// This module utilizes the `csv` dependency to generate reports.
///
mod csv {
    use super::{CliResult, HistoryDates, ReportWriter};
    use csv::Writer;

    /// Generates the list history CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_history_dates` - The list of location and history dates that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(locations_history_dates: Vec<HistoryDates>, report_writer: &ReportWriter) -> CliResult<()> {
        let mut writer = Writer::from_writer(report_writer.get_writer()?);
        writer.write_record(&["location", "start_date", "end_date"])?;
        for location_history_dates in locations_history_dates {
            for history_range in location_history_dates.history_dates {
                let (from, to) = history_range.as_iso8601();
                writer.write_record(&[&location_history_dates.location.name, &from, &to])?;
            }
        }
        Ok(())
    }
}

/// The list history JSON based reporting implementation.
///
/// This module utilizes the `serde_json` dependency to generate reports.
///
mod json {
    use super::*;
    use serde_json::{json, to_string, to_string_pretty, Value};

    /// Generates the list history JSON based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_history_dates` - The list of location and history dates that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    pub fn output(
        locations_history_dates: Vec<HistoryDates>,
        report_writer: &ReportWriter,
        pretty: bool,
    ) -> CliResult<()> {
        let location_array: Vec<Value> = locations_history_dates
            .into_iter()
            .map(|location_history_dates| {
                let history_dates: Vec<Value> = location_history_dates
                    .history_dates
                    .iter()
                    .map(|history_range| {
                        let (from, to) = history_range.as_iso8601();
                        json!({
                            "start": from,
                            "end": to,
                        })
                    })
                    .collect();
                json!({
                    "location": location_history_dates.location.name,
                    "dates": history_dates,
                })
            })
            .collect();
        let root = json!({ "history": location_array });
        let as_text = if pretty { to_string_pretty } else { to_string };
        writeln!(report_writer.get_writer()?, "{}", as_text(&root)?)?;
        Ok(())
    }
}
