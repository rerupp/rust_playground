//! # The implementation for list history (`lh`).
//!
//! The list history command presents what historical weather data is available for a location.
//! The available data is shown as dates. If there are consecutive dates available they will be
//! shown as a range (YYYY-MM-DD to YYYY-MM-DD).
//!
//! The command allows locations_win to be filtered. The filtering is case-insensitive
//! and will match either the start of the location name or alias.
//!
use super::*;

/// The list history command name.
pub const COMMAND_NAME: &'static str = "lh";

pub use v4::{command, execute};
mod v4 {
    //! The current implementation of the list history command.
    use super::*;
    use reports::list_history as reports;

    /// Create the list history command.
    pub fn command() -> Command {
        Command::new(COMMAND_NAME)
            .about("List the dates of weather history available by location.")
            .args(ReportArgs::get())
            .group(ReportArgs::arg_group())
            .args(CriteriaArgs::get())
    }

    /// Executes the list history command.
    ///
    /// # Arguments
    ///
    /// * `weather_data` is the weather library API used by the command.
    /// * `args` contains the report history command arguments.
    ///
    pub fn execute(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
        let histories = weather_data.get_history_dates(DataCriteria {
            filters: CriteriaArgs::new(&args).locations().clone(),
            icase: true,
            sort: true,
        })?;
        match histories.is_empty() {
            true => Ok(()),
            false => {
                let report_args = ReportArgs::new(&args);
                let mut writer = get_writer(&report_args)?;
                let report = if report_args.csv() {
                    reports::csv::Report::default().generate(histories)
                } else if report_args.json() {
                    let report = match report_args.pretty() {
                        true => reports::json::Report::pretty_printed(),
                        false => reports::json::Report::default()
                    };
                    report.generate(histories)
                } else {
                    reports::text::Report::default()
                        .with_title_separator()
                        .with_date_format("%b-%d-%Y")
                        .generate(histories)
                        .into_iter()
                        .map(|row| trim_row_end!(row.to_string()))
                        .collect::<Vec<String>>()
                        .join("\n")
                };
                match writer.write_all(report.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(Error::from(err))
                }
            }
        }
    }
}
