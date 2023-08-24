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
use super::{ReportGenerator, ReportWriter, Result};
use clap::Args;
use weather_lib::prelude::{DataCriteria, Location, WeatherData};

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
    fn get_locations(&self, weather_data: &WeatherData) -> Result<Vec<Location>> {
        let query = DataCriteria { filters: self.args.locations.clone(), icase: true, sort: true };
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
    use super::*;
    use toolslib::{
        rptcols, rptrow,
        text::{write_strings, Report},
    };

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
        let mut report = Report::from(rptcols!(<, <, ^=(long_lat_width * 2 + 1), <));
        report.header(rptrow!(^ "Location", ^ "Alias", ^ "Longitude/Latitude", ^ "Timezone"));
        report.separator("-");
        for location in locations {
            report.text(rptrow!(
                location.name,
                location.alias,
                format!("{:>long_lat_width$}/{:<long_lat_width$}", &location.longitude, &location.latitude),
                location.tz
            ));
        }
        write_strings(&mut report_writer.get_writer()?, report.into_iter())?;
        Ok(())
    }
}

/// The list locations CSV based reporting implementation.
///
/// This module utilizes the `csv` dependency to generate reports.
///
mod csv {
    use super::{Location, ReportWriter, Result};
    use csv::Writer;

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
        let report_writer = report_writer.get_writer()?;
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
    use super::*;
    use serde_json::{json, to_string, to_string_pretty, Value};

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
        writeln!(report_writer.get_writer()?, "{}", as_text(&root)?)?;
        Ok(())
    }
}
