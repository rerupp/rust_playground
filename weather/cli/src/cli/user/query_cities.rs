//! The user command 'qc' to query US Cities location information.
//!
//! qc [-c|--city]=str [-s|--state]=str [-l|--limit]=#
//!
#![allow(unused)]

use super::trim_row_end;
use crate::cli::{self, err, get_writer, reports::list_locations as reports, LocationFilterArgs, ReportArgs};
use clap::{Arg, ArgAction, ArgMatches, Command};
use weather_lib::prelude::{LocationCriteria, LocationFilter, WeatherData, location_filter};

/// The query cities command name.
///
pub const COMMAND_NAME: &'static str = "qc";

const LIMIT: &str = "QUERY_CITIES_LIMIT";

/// create the list summary command.
///
pub fn command() -> Command {
    Command::new(COMMAND_NAME)
        .about("Search cities for location information.")
        .args(LocationFilterArgs::get())
        .args(ReportArgs::get())
        .group(ReportArgs::arg_group())
        .args(vec![
            Arg::new(LIMIT)
                .short('l')
                .long("limit")
                .action(ArgAction::Set)
                .value_name("LIMIT")
                .require_equals(true)
                .value_parser(limit_parser)
                .default_value("30")
                .help("Limit the number of cities shown."),
        ])
}

pub fn execute(weather_data: &WeatherData, args: ArgMatches) -> cli::Result<()> {
    let mut filters = LocationFilterArgs::new(&args).as_location_filters().into_iter().collect::<Vec<_>>();
    let filter = match filters.len() {
        0 => location_filter!(),
        len => {
            if len > 1 {
                println!("Warning: Only one location filter is supported.");
            }
            filters.remove(0)
        },
    };
    let criteria = LocationCriteria {
        filter,
        limit: *args.get_one::<usize>(LIMIT).unwrap(),
    };
    match weather_data.search_locations(criteria) {
        Err(error) => err!("There was an error searching for locations: {:?}", error)?,
        Ok(locations) => match locations.is_empty() {
            true => println!("There were no locations found."),
            false => {
                let report_args = ReportArgs::new(&args);
                let mut writer = get_writer(&report_args)?;
                let report = if report_args.csv() {
                    reports::csv::Report::default().generate(locations)
                } else if report_args.json() {
                    let report = match report_args.pretty() {
                        true => reports::json::Report::pretty_printed(),
                        false => reports::json::Report::default(),
                    };
                    report.generate(locations)
                } else {
                    reports::text::Report::default()
                        .with_skip_alias()
                        .with_title_separator()
                        .generate(&locations)
                        .into_iter()
                        .map(|row| trim_row_end!(row.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                if let Err(error) = writer.write_all(report.as_bytes()) {
                    err!("There was an error writing the locations report: {:?}", error)?;
                }
            }
        },
    }
    Ok(())
}

/// Used by the command parser to make sure the limit is within bounds.
///
/// # Arguments
///
/// * `limit_arg` is the weather directory command argument.
///
fn limit_parser(limit_arg: &str) -> Result<usize, String> {
    match limit_arg.parse::<usize>() {
        Ok(limit) => match limit > 0 {
            true => Ok(limit),
            _ => Err("limit must be greater than 0".to_string()),
        },
        Err(_) => Err("limit needs to be an unsigned integer.".to_string()),
    }
}
