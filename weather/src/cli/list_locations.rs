//! # The implementation for list locations (`ll`).
//!
//! The location information returned by the command includes:
//!
//! * name
//! * alias name
//! * longitude and latitude
//! * timezone
//!
//! The command allows locations to be filtered. The filtering is case insensitive
//! and will match either the start of the location name or alias.
//!
use clap::Args;

use super::lib::{Location, LocationQuery, Locations, WeatherData};
use super::ReportWriter;

use super::{ReportGenerator, Result};

#[derive(Args, Debug)]
/// The command arguments for the list location command.
///
/// The command arguments that determine which locations will be included in the report. All
/// locations will be used in the report unless specific locations are selected.
pub struct CommandArgs {
    /// Filter output to these locations (Optional).
    locations: Vec<String>,
}

/// The data associated with a list locations command.
pub struct ListLocations {
    /// The list locations command arguments.
    args: CommandArgs,
}

impl ListLocations {
    /// Create a new instance of the list location command.
    ///
    /// # Arguments
    ///
    /// * `args` - the command arguments association with the instance.
    ///
    pub fn new(args: CommandArgs) -> ListLocations {
        ListLocations { args }
    }

    /// Get the weather data locations.
    ///
    /// # Arguments
    ///
    /// `weather_data` - the `domain` instance that will be used.
    ///
    fn get_locations(&self, weather_data: &WeatherData) -> Result<Locations> {
        let query = LocationQuery {
            location_filter: self.args.locations.clone(),
            case_insensitive: true,
            sort: true,
        };
        Ok(weather_data.get_locations(query)?)
    }
}

/// The implementation of the `ReportGenerator` trait for list locations.
impl ReportGenerator for ListLocations {
    /// Generates a text based report for list locations.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn text_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> Result<()> {
        text::output(self.get_locations(weather_data)?, report_writer)
    }
    /// Generates a JSON report for list locations.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    fn json_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> Result<()> {
        json::output(self.get_locations(weather_data)?, report_writer, pretty)
    }
    /// Generates a CSV report for list locations.
    ///
    /// An error will be returned if there are issues getting locations from the domain.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    fn csv_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> Result<()> {
        csv::output(self.get_locations(weather_data)?, report_writer)
    }
}

/// The list locations text based reporting implementation.
///
/// This module utilizes the `text_reports` module to generate reports.
///
mod text {
    use toolslib::text::{Alignment, ColumnDescription, Columns, Report, Row, RowID};

    use super::*;

    /// Generates the list locations text based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `locations` - The list of locations that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(locations: Vec<Location>, report_writer: &ReportWriter) -> Result<()> {
        let long_lat_width = "-###.########".len();
        let mut report = Report::from(vec![
            ColumnDescription::from(Alignment::Left),
            ColumnDescription::from(Alignment::Left),
            ColumnDescription::from(Alignment::Center).with_width(long_lat_width * 2 + 1),
            ColumnDescription::from(Alignment::Left),
        ])
        .with_rows(vec![
            Row::from(RowID::Header)
                .with_alignment(Alignment::Center)
                .with_columns(Columns::from(vec!["Location", "Alias", "Longitude/Latitude", "Timezone"])),
            Row::from(RowID::Separator('-')),
        ])?;
        for location in locations {
            report.add(Row::from(RowID::Detail).with_columns(Columns::from(vec![
                location.name.as_str(),
                location.alias.as_str(),
                format!("{:>long_lat_width$}/{:<long_lat_width$}", &location.longitude, &location.latitude).as_str(),
                location.tz.as_str(),
            ])))?;
        }
        report.generate(report_writer.create()?)?;
        Ok(())
    }
}

/// The list locations CSV based reporting implementation.
///
/// This module utilizes the `csv` dependency to generate reports.
///
mod csv {
    use csv::Writer;

    use crate::cli::ReportWriter;

    use super::{Location, Result};

    /// Generates the list locations CSV based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `locations` - The list of locations that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    ///
    pub fn output(locations: Vec<Location>, report_writer: &ReportWriter) -> Result<()> {
        let report_writer = report_writer.create()?;
        let mut writer = Writer::from_writer(report_writer);
        writer.write_record(&["name", "longitude", "latitude", "alias", "tz"])?;
        for location in locations {
            writer.write_record(&[
                location.name,
                location.longitude,
                location.latitude,
                location.alias,
                location.tz,
            ])?;
        }
        Ok(())
    }
}

/// The list locations JSON based reporting implementation.
///
/// This module utilizes the `serde_json` dependency to generate reports.
///
mod json {
    use serde_json::{json, to_string, to_string_pretty, Value};

    use super::*;

    /// Generates the list locations JSON based report.
    ///
    /// An error will be returned if there are issues writing the report.
    ///
    /// # Arguments
    ///
    /// * `locations` - The list of locations that will be reported.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    pub fn output(locations: Vec<Location>, report_writer: &ReportWriter, pretty: bool) -> Result<()> {
        let location_array: Vec<Value> = locations
            .iter()
            .map(|location| {
                json!({
                    "name": location.name,
                    "alias": location.alias,
                    "longitude": location.longitude,
                    "latitude": location.latitude,
                    "tz": location.tz
                })
            })
            .collect();
        let root = json!({ "locations": location_array });
        let as_text = if pretty { to_string_pretty } else { to_string };
        writeln!(report_writer.create()?, "{}", as_text(&root)?)?;
        Ok(())
    }
}
