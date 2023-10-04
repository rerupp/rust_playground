//! # The implementation for list history (`lh`).
//!
//! The list history command presents what historical weather data is available for a location.
//! The available data is shown as dates. If there are consecutive dates available they will be
//! shown as a range (ie. YYYY-MM-DD to YYYY-MM-DD).
//!
//! The command allows locations to be filtered. The filtering is case insensitive
//! and will match either the start of the location name or alias.
use super::{get_writer, ListHistory, Result};
use std::io::Write;
use weather_lib::prelude::{DataCriteria, HistoryDates, WeatherData};

pub(in crate::cli) fn execute(weather_data: &WeatherData, cmd_args: ListHistory) -> Result<()> {
    let histories = weather_data.get_history_dates(DataCriteria {
        filters: cmd_args.criteria_args().locations().clone(),
        icase: true,
        sort: true,
    })?;
    let report_args = cmd_args.report_args();
    let mut writer = get_writer(&report_args)?;
    if report_args.csv() {
        csv_report::generate(histories, &mut writer)
    } else if report_args.json() {
        json_report::generate(histories, &mut writer, report_args.pretty())
    } else {
        text_report::generate(histories, &mut writer)
    }
}

mod text_report {
    /// The list history text based reporting implementation.
    ///
    /// This module utilizes the `text_reports` module to generate reports.
    ///
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
    /// * `writer` - The output manager that controls where report output will be sent.
    pub fn generate(locations_history_dates: Vec<HistoryDates>, writer: &mut impl Write) -> Result<()> {
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
        write_strings(writer, report.into_iter())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}

mod csv_report {
    /// The list history CSV based reporting implementation.
    ///
    /// This module utilizes the `csv` dependency to generate reports.
    ///
    use super::*;
    use csv::Writer;

    /// Generates the list history CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_history_dates` - The list of location and history dates that will be reported.
    /// * `writer` - The output manager that controls where report output will be sent.
    ///
    pub fn generate(locations_history_dates: Vec<HistoryDates>, writer: &mut impl Write) -> Result<()> {
        let mut writer = Writer::from_writer(writer);
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

mod json_report {
    /// The list history JSON based reporting implementation.
    ///
    /// This module utilizes the `serde_json` dependency to generate reports.
    ///
    use super::*;
    use serde_json::{json, to_string, to_string_pretty, Value};

    /// Generates the list history JSON based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_history_dates` - The list of location and history dates that will be reported.
    /// * `writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    pub fn generate(locations_history_dates: Vec<HistoryDates>, writer: &mut impl Write, pretty: bool) -> Result<()> {
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
        writeln!(writer, "{}", as_text(&root)?)?;
        Ok(())
    }
}
