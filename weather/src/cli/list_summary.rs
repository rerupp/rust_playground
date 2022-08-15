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
use clap::Args;

use crate::domain::{LocationHistorySummaries, LocationQuery, WeatherData};

use super::{CliResult, ReportGenerator, ReportWriter};

#[derive(Args, Debug)]
/// The command arguments for the list summary command.
pub struct CommandArgs {
    /// Filter output to these locations (Optional).
    locations: Vec<String>,
}

/// The data associated with a list locations command.
pub struct ListSummary {
    /// The list locations command arguments.
    args: CommandArgs,
}

impl ListSummary {
    /// Create a new instance of the list summary command.
    ///
    /// # Arguments
    ///
    /// * `args` - the command arguments association with the instance.
    ///
    pub fn new(args: CommandArgs) -> ListSummary {
        ListSummary { args }
    }

    /// Get the locations history summary.
    ///
    /// # Arguments
    ///
    /// `weather_data` - the `domain` instance that will be used.
    ///
    fn get_history_summary(&self, weather_data: &WeatherData) -> CliResult<LocationHistorySummaries> {
        let query = LocationQuery {
            location_filter: self.args.locations.clone(),
            case_insensitive: true,
            sort: true,
        };
        Ok(weather_data.get_history_summary(query)?)
    }
}

/// The implementation of the `ReportGenerator` trait for list locations.
impl ReportGenerator for ListSummary {
    /// Generates a text based report for list summary.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn text_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
        text::output(self.get_history_summary(weather_data)?, report_writer)
    }

    /// Generates a JSON report for list summary.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    fn json_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> CliResult<()> {
        json::output(self.get_history_summary(weather_data)?, report_writer, pretty)
    }

    /// Generates a CSV report for list summary.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn csv_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()> {
        csv::output(self.get_history_summary(weather_data)?, report_writer)
    }
}

/// The list summary text based reporting implementation.
///
/// This module utilizes the `text_reports` module to generate reports.
///
mod text {
    use thousands::Separable;

    use crate::cli::text_reports::{Alignment, ColumnContent, ColumnProperty, ReportBuilder, ReportColumns};

    use super::*;

    /// Generates the locations summary text based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_histories` - The list of location history summaries that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(location_histories: LocationHistorySummaries, report_writer: &ReportWriter) -> CliResult<()> {
        let mut report_builder = ReportBuilder::new(ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Left),
            ColumnProperty::new(Alignment::Right),
            ColumnProperty::new(Alignment::Right),
            ColumnProperty::new(Alignment::Right),
            ColumnProperty::new(Alignment::Right),
        ]));
        report_builder.add_contents(ReportBuilder::header().with_contents(vec![
            ColumnContent::new("Location").with_alignment(Alignment::Center),
            ColumnContent::new("Overall Size").with_alignment(Alignment::Center),
            ColumnContent::new("History Count").with_alignment(Alignment::Center),
            ColumnContent::new("Raw History Size").with_alignment(Alignment::Center),
            ColumnContent::new("Compressed Size").with_alignment(Alignment::Center),
        ]))?;
        report_builder.add_separator('-');
        let mut total_size = 0;
        let mut total_history_count = 0;
        let mut total_raw_size = 0;
        let mut total_compressed_size = 0;
        for (location, history_summary) in location_histories {
            let overall_size = history_summary.overall_size.unwrap_or(0);
            let raw_size = history_summary.raw_size.unwrap_or(0);
            let compressed_size = history_summary.compressed_size.unwrap_or(0);
            report_builder.add_contents(ReportBuilder::detail().with_contents(vec![
                ColumnContent::new(&location.name),
                ColumnContent::new(&to_kib(overall_size)),
                ColumnContent::new(&history_summary.count.separate_with_commas()),
                ColumnContent::new(&to_kib(raw_size)),
                ColumnContent::new(&to_kib(compressed_size)),
            ]))?;
            total_size += overall_size;
            total_history_count += history_summary.count;
            total_raw_size += raw_size;
            total_compressed_size += compressed_size;
        }
        report_builder.add_separator('=');
        report_builder.add_contents(ReportBuilder::detail().with_contents(vec![
            ColumnContent::new("Total"),
            ColumnContent::new(&to_kib(total_size)),
            ColumnContent::new(&total_history_count.separate_with_commas()),
            ColumnContent::new(&to_kib(total_raw_size)),
            ColumnContent::new(&to_kib(total_compressed_size)),
        ]))?;
        report_builder.output(report_writer)
    }

    /// Converts the size value into a string.
    ///
    /// The returned value be represented in kilobytes unless the value is less than 1024. Examples
    /// follow:
    ///
    /// | Size | Result |
    /// |  ---:|    ---:|
    /// |    0 |     0  |
    /// |  999 |   999  |
    /// | 1023 | 1,023  |
    /// | 1024 | 1 KiB  |
    /// | 1024000 | 1,000 KiB  |
    ///
    pub fn to_kib(size: usize) -> String {
        let kib = ((size as f64 / 1024.0) + 0.5) as usize;
        let mut pretty_kib = kib.separate_with_commas();
        if size >= 1024 {
            pretty_kib.push_str(" Kib");
        }
        pretty_kib
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn kib() {
            assert_eq!(to_kib(511), "0".to_string());
            assert_eq!(to_kib(512), "1".to_string());
            assert_eq!(to_kib(1024), "1 Kib".to_string());
            assert_eq!(to_kib(1024 * 1024), "1,024 Kib".to_string());
        }
    }
}

/// The list summary CSV based reporting implementation.
///
/// This module utilizes the `csv` dependency to generate reports.
///
mod csv {
    use csv::Writer;

    use crate::cli::ReportWriter;

    use super::{CliResult, LocationHistorySummaries};

    /// Generates the list summary CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `location_histories` - The list of location history summaries that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(location_histories: LocationHistorySummaries, report_writer: &ReportWriter) -> CliResult<()> {
        let mut writer = Writer::from_writer(report_writer.create()?);
        writer.write_record(&["location", "entries", "entries_size", "compressed_size", "size"])?;
        for (location, history) in location_histories {
            let raw_size = history.raw_size.map_or(0, |v| v);
            let compressed_size = history.compressed_size.map_or(0, |v| v);
            let overall_size = history.overall_size.map_or(0, |v| v);
            writer.write_record(&[
                location.name,
                history.count.to_string(),
                raw_size.to_string(),
                compressed_size.to_string(),
                overall_size.to_string()
            ])?;
        }
        Ok(())
    }
}

/// The list summary JSON based reporting implementation.
///
/// This module utilizes the `serde_json` dependency to generate reports.
///
mod json {
    use serde_json::{json, to_string, to_string_pretty, Value};

    use super::*;

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
    pub fn output(location_histories: LocationHistorySummaries, report_writer: &ReportWriter, pretty: bool) -> CliResult<()> {
        let location_array: Vec<Value> = location_histories
            .iter()
            .map(|location_history| {
                let (location, history) = location_history;
                json!({
                    "location": location.name,
                    "entries": history.count,
                    "entries_size": history.raw_size.map_or(0, |v| v),
                    "compressed_size": history.compressed_size.map_or(0, |v| v),
                    "size": history.overall_size.map_or(0, |v| v),
                })
            })
            .collect();
        let root = json!({
            "locations": location_array
        });
        let to_text = if pretty { to_string_pretty } else { to_string };
        writeln!(report_writer.create()?, "{}", to_text(&root)?)?;
        Ok(())
    }
}