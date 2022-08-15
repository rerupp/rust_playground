//! # The weather CLI.
//!
//! The `cli` module provides a front end command line interface (CLI) to weather data. It uses the
//! `domain` module API to access weather data.
//!
//! The CLI is built upon `clap` and uses *derive* syntax to define subcommands. The CLI currently
//! hosts 4 subcommands.

use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;

use clap::{AppSettings, Args, Parser, Subcommand};

use list_locations as ll;

use crate::core;
use crate::domain::WeatherData;

mod list_locations;
mod list_summary;
mod list_history;
mod report_history;
mod text_reports;

/// The Result returned from the cli module
///
/// Today the result is simply an alias to the *core* `WeatherResult` but this
/// could change in the future.
pub type CliResult<T> = core::WeatherResult<T>;

/// An error that can be returned from the cli module.
///
/// Today the error is simple an alias to the *core* `WeatherError` but this
/// could change in the future.
pub type CliError = core::WeatherError;

/// The CLI commands available for weather data.
///
/// These commands provide the implementation of the CLI interface.
#[derive(Parser, Debug)]
#[clap(author, version, setting = AppSettings::ArgRequiredElseHelp)]
pub struct Cli {
    /// The directory pathname containing weather data.
    ///
    /// The data directory  is optional. If it is not provided a default will be used.
    #[clap(short, long = "directory", value_parser)]
    pub data_dir: Option<String>,

    /// The command that will be run.
    ///
    /// The command is optional. If not provided a summary of the available commands
    /// will be displayed.
    #[clap[subcommand]]
    pub command: Option<Commands>,
}

impl Cli {
    /// Get the data directory that is common to many of the commands.
    pub fn data_dir(&self) -> &str {
        match &self.data_dir {
            Some(s) => s.as_str(),
            None => ""
        }
    }
}

/// The commands providing access to weather data.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show weather data locations.
    ///
    /// The *ll* command reports information about the weather data locations
    /// that are available.
    #[clap(name = "ll", setting = AppSettings::DeriveDisplayOrder)]
    ListLocations {
        /// The list locations command arguments.
        #[clap(flatten)]
        args: ll::CommandArgs,
        /// The type of report that will be generated.
        #[clap(flatten)]
        report_type: ReportType,
    },

    /// Show a summary of weather data available by location.
    ///
    /// The *ls* command reports a summary of weather data by location. The report
    /// includes totals for the selected (if any) locations.
    #[clap(name = "ls", setting = AppSettings::DeriveDisplayOrder)]
    ListSummary {
        /// The list summary command arguments.
        #[clap(flatten)]
        args: list_summary::CommandArgs,
        /// The type of report that will be generated.
        #[clap(flatten)]
        report_type: ReportType,
    },

    /// List weather data, by date, available by location.
    ///
    /// The *lh* command reports weather data date ranges by location.
    #[clap(name = "lh", setting = AppSettings::DeriveDisplayOrder)]
    ListHistory {
        /// The list history command arguments.
        #[clap(flatten)]
        args: list_history::CommandArgs,
        /// The type of report that will be generated.
        #[clap(flatten)]
        report_type: ReportType,
    },

    /// Generate a weather data report for a location.
    ///
    /// the *rh* command generates a report of weather data by location.
    #[clap(name = "rh", setting = AppSettings::DeriveDisplayOrder)]
    ReportHistory {
        /// The report history command arguments.
        #[clap(flatten)]
        args: report_history::CommandArgs,
        /// The type of report that will be generated.
        #[clap(flatten)]
        report_type: ReportType,
    },
}

/// The report generator is implemented by commands that produce reports.
pub trait ReportGenerator<> {
    /// Generate a report as plain text.
    ///
    /// The output is formatted in tabular form. Column headers are included that help
    /// identify what data is being is being reported. The report text is human readable.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    fn text_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()>;

    /// Generate a report in JSON format.
    ///
    /// The output will be formatted as JSON text. The JSON text will be void of insignificant
    /// white space unless pretty is true. When pretty is true output will contain indentation
    /// and the document will be more human friendly.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    /// * `pretty` - if `true` JSON output will be formatted with space and newlines.
    ///
    fn json_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> CliResult<()>;

    /// Generate a report in CSV format.
    ///
    /// The CSV data is separated by the ***,*** character. At some point a command line argument
    /// may be added to control this behavior.
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    fn csv_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> CliResult<()>;
}

/// This defines arguments common to commands that produce reports.
///
/// The report type can be one of the following.
///
/// * *text* - the generated report will be formatted as text (default).
/// * *json* - the generated report will be formatted as JSON.
/// * *csv* - the generated report will be formatted as CSV.
///
/// Only one of the flags can be set by the CLI parsing.
///
/// There is an optional *filename* argument that directs the CLI to output a report
/// to a file. If it is not provided output will go to the console window.
///
/// There is a pretty argument used by the JSON reports to indicate the output should be formatted
/// as lines with indentation to make it easier for read by humans.
///
#[derive(Args, Debug)]
pub struct ReportType {
    /// The output will be plain Text (default).
    #[clap(long, value_parser, group = "report_type")]
    pub text: bool,
    /// The output will be in CSV format.
    #[clap(long, value_parser, group = "report_type")]
    pub csv: bool,
    /// The output will be in JSON format.
    #[clap(long, value_parser, group = "report_type")]
    pub json: bool,
    /// The report output file pathname, if not provided output is directed to *stdout*.
    #[clap(short = 'f', long = "file", value_name = "FILENAME", forbid_empty_values = true, parse(try_from_str = parse_filename))]
    pub filepath: Option<PathBuf>,
    /// This flag is specific to JSON reports and indicates reports should be more human readable.
    #[clap(short, long, conflicts_with_all = & ["text", "csv"])]
    pub pretty: bool,
}

impl ReportType {
    /// Dispatch the type of report generated by a command.
    ///
    /// A text report will be generated by default if a report type is not included on the command
    /// line. The report writer is created here before calling the appropriate generator method.
    ///
    /// # Arguments
    ///
    /// * `report_generator` - the command for which a report will be generated.
    /// * `weather_data` - The domain API used to access weather data.
    pub fn dispatch(&self, report_generator: &impl ReportGenerator, weather_data: &WeatherData) -> CliResult<()> {
        let report_writer = ReportWriter::new(&self.filepath);
        match (self.text, self.csv, self.json) {
            (false, false, false) => report_generator.text_output(weather_data, &report_writer),
            (true, false, false) => report_generator.text_output(weather_data, &report_writer),
            (false, true, false) => report_generator.csv_output(weather_data, &report_writer),
            (false, false, true) => report_generator.json_output(weather_data, &report_writer, self.pretty),
            _ => Err(CliError::new("The report type is not valid..."))
        }
    }
}

/// Manages where a generated report will be written to.
///
/// If the command line includes a filename argument, generated output will be written to it
/// otherwise output will be sent to *stdout*. The writer is reusable.
///
pub struct ReportWriter<'a>(&'a Option<PathBuf>);

impl ReportWriter<'_> {
    /// Creates an instance of the report writer container.
    ///
    /// # Arguments
    ///
    /// * `pathbuf_option` - the optional file pathname where generated reports will be written.
    ///
    pub fn new(pathbuf_option: &Option<PathBuf>) -> ReportWriter {
        ReportWriter(pathbuf_option)
    }

    /// Creates `Write` instance where reports can be written.
    ///
    /// If the report writer contains a file pathname, an error can occur due to permission
    /// or locking issues.
    pub fn create(&self) -> CliResult<Box<dyn Write>> {
        match &self.0 {
            None => Ok(Box::new(io::stdout()) as Box<dyn Write>),
            Some(filepath) => {
                let pathname = filepath.deref().to_str().unwrap();
                match OpenOptions::new().write(true).create(true).truncate(true).open(pathname) {
                    Err(error) => Err(CliError::new(&format!("{}", error.to_string()))),
                    Ok(file_writer) => Ok(Box::new(file_writer) as Box<dyn Write>),
                }
            }
        }
    }
}

/// A filename parser used by the CLI.
///
/// The parser is responsible for creating a `Path` from the string argument. The following
/// rudimentary checks are performed which may result in an error being returned.
///
/// * Check if the filename points to a directory.
/// * Check if the filename is a symlink to another file.
/// * Check if the path (if present) exists.
///
/// # Arguments
///
/// * `filename` - the filename as entered on the command line.
///
fn parse_filename(filename: &str) -> Result<PathBuf, String> {
    let filepath = PathBuf::from(filename);
    if filepath.is_dir() {
        Err(format!("{} is a directory...", filename))
    } else if filepath.is_symlink() {
        Err(format!("{} is a symlink...", filename))
    } else if filepath.is_absolute() && !filepath.parent().unwrap().exists() {
        Err(format!("The parent directory does not exist..."))
    } else {
        // you can read all about this but "bar.txt" and "foo/bar.txt" are both relative AND
        // have parent paths, one just happens to be empty...
        let parent = filepath.parent().unwrap();
        if parent.to_str().unwrap().len() > 0 && !parent.exists() {
            Err(format!("The relative path to file does not exist..."))
        } else {
            Ok(filepath)
        }
    }
}

/// Map the cli command to the implementation.
///
/// This method is responsible for creating an instance of the subcommand. After
/// creating the command it is dispatched to generate the appropriate type of report.
///
/// # Arguments
///
/// * `cli` - An instance of the CLI after the command line has been successfully parsed.
/// * `weather_data` - The domain API used to access weather data.
pub fn dispatch(cli: Cli, weather_api: &WeatherData) -> CliResult<()> {
    match cli.command {
        Some(Commands::ListLocations { args, report_type }) => {
            let list_locations = ll::ListLocations::new(args);
            report_type.dispatch(&list_locations, &weather_api)
        }
        Some(Commands::ListSummary { args, report_type }) => {
            let list_summary = list_summary::ListSummary::new(args);
            report_type.dispatch(&list_summary, &weather_api)
        }
        Some(Commands::ListHistory { args, report_type }) => {
            let list_history = list_history::ListHistory::new(args);
            report_type.dispatch(&list_history, &weather_api)
        }
        Some(Commands::ReportHistory { args, report_type }) => {
            let report_history = report_history::ReportHistory::new(args);
            report_type.dispatch(&report_history, &weather_api)
        }
        _ => Err(CliError::new("Unsupported command!!!"))
    }
}

// I kind of wanted to hang onto this in case it was needed in the future. Right
// now it isn't but...
#[doc(hidden)]
pub mod util {
    use std::cmp;

    pub fn max<T, F: Fn(&T) -> usize, I: IntoIterator<Item=T>>(i: I, f: F) -> usize {
        let mut max_width: usize = 0;
        i.into_iter().for_each(|item| {
            max_width = cmp::max(max_width, f(&item))
        });
        max_width
    }

    #[cfg(test)]
    mod util_tests {
        use super::*;

        #[test]
        fn verify_max() {
            let test_case = vec![0, 5, 10, 3, 7];
            assert_eq!(10, max(test_case, |v| *v));
        }
    }
}
