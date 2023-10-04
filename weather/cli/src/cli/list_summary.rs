//! # The implementation for list summary (`ls`).
//!
//! The list summary command presents the amount of weather data available. The information
//! includes:
//!
//! * location name
//! * the count of how many historical weather data entries there are
//! * the overall size of weather data
//! * the total size of raw data
//! * the size of the data when compressed
//!
//! The command allows locations to be filtered. The filtering is case insensitive
//! and will match either the start of the location name or alias.
//!
use super::{ListSummary, Result, get_writer};
use weather_lib::prelude::{DataCriteria, HistorySummaries, WeatherData};
use std::io::Write;

pub(in crate::cli) fn execute(weather_data: &WeatherData, cmd_args: ListSummary) -> Result<()> {
    let history_summaries = weather_data.get_history_summary(DataCriteria {
        filters: cmd_args.criteria_args().locations().clone(),
        icase: true,
        sort: true,
    })?;
    let report_args = cmd_args.report_args();
    let mut writer = get_writer(&report_args)?;
    if report_args.csv() {
        csv_report::generate(history_summaries, &mut writer)
    } else if report_args.json() {
        json_report::generate(history_summaries, &mut writer, report_args.pretty())
    } else {
        text_report::generate(history_summaries, &mut writer)
    }
}

mod text_report {
    /// The list summary text based reporting implementation.
    ///
    /// This module utilizes the `text_reports` module to generate reports.
    ///
    use super::*;
    use toolslib::{
        fmt::commafy,
        kib, rptcols, rptrow,
        text::{write_strings, Report},
    };

    /// Generates the locations summary text based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_histories` - The list of location history summaries that will be reported.
    /// * `writer` - The output manager that controls where report output will be sent.
    ///
    pub fn generate(location_histories: Vec<HistorySummaries>, writer: &mut impl Write) -> Result<()> {
        let mut report = Report::from(rptcols!(<, >, >, >, >));
        report.header(rptrow!(^"Location", ^"Overall Size", ^"History Count", ^"History Size", ^"Store Size"));
        report.separator("-");
        let mut total_size = 0;
        let mut total_history_count = 0;
        let mut total_raw_size = 0;
        let mut total_compressed_size = 0;
        for location_history_summary in location_histories {
            let overall_size = location_history_summary.overall_size.unwrap_or(0);
            let raw_size = location_history_summary.raw_size.unwrap_or(0);
            let compressed_size = location_history_summary.store_size.unwrap_or(0);
            report.text(rptrow!(
                location_history_summary.location.name,
                kib!(overall_size, 0),
                commafy(location_history_summary.count),
                kib!(raw_size, 0),
                kib!(compressed_size, 0)
            ));
            total_size += overall_size;
            total_history_count += location_history_summary.count;
            total_raw_size += raw_size;
            total_compressed_size += compressed_size;
        }
        report.separator("=").text(rptrow!(
            "Total",
            kib!(total_size, 0),
            commafy(total_history_count),
            kib!(total_raw_size, 0),
            kib!(total_compressed_size, 0),
        ));
        write_strings(writer, report.into_iter())?;
        Ok(())
    }
}

mod csv_report {
    /// The list summary CSV based reporting implementation.
    ///
    /// This module utilizes the `csv` dependency to generate reports.
    ///
    use super::*;
    use csv::Writer;

    /// Generates the list summary CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_histories` - The list of location history summaries that will be reported.
    /// * `writer` - The output manager that controls where report output will be sent.
    ///
    pub fn generate(locations_history_summary: Vec<HistorySummaries>, writer: &mut impl Write) -> Result<()> {
        let mut writer = Writer::from_writer(writer);
        writer.write_record(&["location", "entries", "entries_size", "compressed_size", "size"])?;
        for location_history_summary in locations_history_summary {
            let raw_size = location_history_summary.raw_size.map_or(0, |v| v);
            let compressed_size = location_history_summary.store_size.map_or(0, |v| v);
            let overall_size = location_history_summary.overall_size.map_or(0, |v| v);
            writer.write_record(&[
                location_history_summary.location.name,
                location_history_summary.count.to_string(),
                raw_size.to_string(),
                compressed_size.to_string(),
                overall_size.to_string(),
            ])?;
        }
        Ok(())
    }
}

mod json_report {
    /// The list summary JSON based reporting implementation.
    ///
    /// This module utilizes the `serde_json` dependency to generate reports.
    ///
    use super::*;
    use serde_json::{json, to_string, to_string_pretty, Value};

    /// Generates the list summary JSON based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_histories` - The list of location history summaries that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    pub fn generate(location_histories: Vec<HistorySummaries>, writer: &mut impl Write, pretty: bool) -> Result<()> {
        let location_array: Vec<Value> = location_histories
            .into_iter()
            .map(|location_history_summary| {
                json!({
                    "location": location_history_summary.location.name,
                    "entries": location_history_summary.count,
                    "entries_size": location_history_summary.raw_size.map_or(0, |v| v),
                    "compressed_size": location_history_summary.store_size.map_or(0, |v| v),
                    "size": location_history_summary.overall_size.map_or(0, |v| v),
                })
            })
            .collect();
        let root = json!({ "locations": location_array });
        let to_text = if pretty { to_string_pretty } else { to_string };
        writeln!(writer, "{}", to_text(&root)?)?;
        Ok(())
    }
}
