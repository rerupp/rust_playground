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
//! The command allows locations_win to be filtered. The filtering is case-insensitive
//! and will match either the start of the location name or alias.
//!
use super::*;

/// The list summary command name.
pub const COMMAND_NAME: &'static str = "ls";

pub use v4::{command, execute};
mod v4 {
    //! The current implementation of the list summary command.
    use super::*;
    use reports::list_summary as reports;

    /// create the list summary command.
    pub fn command() -> Command {
        Command::new(COMMAND_NAME)
            .about("List a summary of weather data available by location.")
            .args(ReportArgs::get())
            .group(ReportArgs::arg_group())
            .args(CriteriaArgs::get())
    }

    /// Executes the list summary command.
    ///
    /// # Arguments
    ///
    /// * `weather_data` is the weather library API used by the command.
    /// * `args` contains the list summary command arguments.
    pub fn execute(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
        let history_summaries = weather_data.get_history_summary(DataCriteria {
            filters: CriteriaArgs::new(&args).locations().clone(),
            icase: true,
            sort: true,
        })?;
        match history_summaries.is_empty() {
            true => Ok(()),
            false => {
                let report_args = ReportArgs::new(&args);
                let report = if report_args.csv() {
                    reports::csv::Report::default().generate(history_summaries)
                } else if report_args.json() {
                    let report = match report_args.pretty() {
                        true => reports::json::Report::pretty_printed(),
                        false => reports::json::Report::default(),
                    };
                    report.generate(history_summaries)
                } else {
                    reports::text::Report::default()
                        .with_title_separator()
                        .generate(history_summaries)
                        .into_iter()
                        .map(|row| trim_row_end!(row.to_string()))
                        .collect::<Vec<String>>()
                        .join("\n")
                };
                let mut writer = get_writer(&report_args)?;
                match writer.write_all(report.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(Error::from(err))
                }
            }
        }
    }
}
