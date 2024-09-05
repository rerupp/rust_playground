//! The list location command implementation.
use super::*;


/// The list locations command name.
pub const COMMAND_NAME: &'static str = "ll";

pub use v4::{command, execute};
mod v4 {
    //! # The implementation for list locations_win (`ll`).
    //!
    //! The location information returned by the command includes:
    //!
    //! * name
    //! * alias name
    //! * longitude and latitude
    //! * timezone
    //!
    //! The command allows locations to be filtered. The filtering is case-insensitive
    //! and will match either the start of the location name or alias.
    //!
    use super::*;
    use reports::list_locations as reports;

    /// Create the list locations command.
    ///
    pub fn command() -> Command {
        Command::new(COMMAND_NAME)
            .about("List the known weather data history locations_win.")
            .args(ReportArgs::get())
            .group(ReportArgs::arg_group())
            .args(CriteriaArgs::get())
    }

    /// Executes the list locations command.
    ///
    /// # Arguments
    ///
    /// * `weather_data` is the weather library API used by the command.
    /// * `args` contains the list locations command arguments.
    ///
    pub fn execute(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
        let locations = weather_data.get_locations(DataCriteria {
            filters: CriteriaArgs::new(&args).locations().clone(),
            icase: true,
            sort: true,
        })?;
        match locations.is_empty() {
            true => Ok(()),
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
                        .with_title_separator()
                        .generate(&locations)
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
