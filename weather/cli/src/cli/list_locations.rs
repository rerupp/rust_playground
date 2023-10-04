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
use super::{ListLocations, get_writer, Result};
use weather_lib::prelude::{DataCriteria, Location, WeatherData};
use std::io::Write;

pub(in crate::cli) fn execute(weather_data: &WeatherData, cmd_args: ListLocations) -> Result<()> {
    let locations = weather_data.get_locations(DataCriteria {
        filters: cmd_args.criteria_args().locations().clone(),
        icase: true,
        sort: true,
    })?;
    let report_args = cmd_args.report_args();
    let mut writer = get_writer(&report_args)?;
    if report_args.csv() {
        csv_report::generate(locations, &mut writer)
    } else if report_args.json() {
        json_report::generate(locations, &mut writer, report_args.pretty())
    } else {
        text_report::generate(locations, &mut writer)
    }
}

mod text_report {
    /// The list locations text based reporting implementation.
    ///
    /// This module utilizes the `text_reports` module to generate reports.
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
    pub fn generate(locations: Vec<Location>, writer: &mut impl Write) -> Result<()> {
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
        write_strings(writer, report.into_iter())?;
        Ok(())
    }
}

mod csv_report {
    /// The list locations CSV based reporting implementation.
    ///
    /// This module utilizes the `csv` dependency to generate reports.
    use super::*;
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
    pub fn generate(locations: Vec<Location>, writer: &mut impl Write) -> Result<()> {
        let mut writer = Writer::from_writer(writer);
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

mod json_report {
    /// The list locations JSON based reporting implementation.
    ///
    /// This module utilizes the `serde_json` dependency to generate reports.
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
    pub fn generate(locations: Vec<Location>, writer: &mut impl Write, pretty: bool) -> Result<()> {
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
        writeln!(writer, "{}", as_text(&root)?)?;
        Ok(())
    }
}
