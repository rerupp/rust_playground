//! # The weather CLI.
//!
//! The `cli` module provides a front end command line interface (CLI) to weather data. It uses the
//! `domain` module API to access weather data.
//!
//! The CLI is built upon `clap` and uses *derive* syntax to define subcommands. The CLI currently
//! hosts 4 subcommands.

use super::lib::{self, WeatherData};
use clap::{AppSettings, ArgAction, Args, Parser, Subcommand};
use std::{fmt::Display, io, path::PathBuf, result};
use toolslib::{
    date_time::parse_date,
    text::{get_writer, write_strings, Report},
    Error as ToolsLibError,
};

mod list_history;
mod list_locations;
mod list_summary;
mod report_history;

pub type Result<T> = result::Result<T, Error>;

/// The CLI error definition.
#[derive(Debug)]
pub struct Error(String);
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(error.as_str())
    }
}
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(format!("cli: {error}"))
    }
}
impl From<lib::Error> for Error {
    fn from(error: lib::Error) -> Self {
        Error(String::from(&error))
    }
}
impl From<ToolsLibError> for Error {
    fn from(error: ToolsLibError) -> Self {
        Error(error.to_string())
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error(format!("io: {error}"))
    }
}
impl From<csv::Error> for Error {
    fn from(error: csv::Error) -> Self {
        Error(format!("csv: {error}"))
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error(format!("json: {error}"))
    }
}
impl From<toolslib::text::Error> for Error {
    fn from(error: toolslib::text::Error) -> Self {
        Error(error.to_string())
    }
}

/// The CLI commands available for weather data.
///
/// These commands provide the implementation of the CLI interface.
#[derive(Parser, Debug)]
#[clap(author, version, setting = AppSettings::ArgRequiredElseHelp)]
pub struct Cli {
    /// The directory pathname containing weather data.
    ///
    /// The data directory  is optional. If it is not provided a default will be used.
    #[clap(short, long = "directory", value_parser, display_order = 1)]
    pub data_dir: Option<String>,
    /// The filename logging output will be written into
    #[clap(
        long = "log", value_name="LOG",
        forbid_empty_values = true, parse(try_from_str = parse_filename),
        group="log", display_order = 2
    )]
    pub logfile_path: Option<PathBuf>,
    /// Append to the log file, otherwise overwrite
    #[clap(short, long = "append", requires("log"), display_order = 3)]
    pub append_log: bool,
    /// Logging verbosity level (once=INFO, twice=DEBUG, thrice=TRACE)
    #[clap(short, long, action(ArgAction::Count), display_order = 4)]
    pub verbosity: u8,
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
            None => "",
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
        args: list_locations::CommandArgs,
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
pub trait ReportGenerator {
    /// Generate a report as plain text.
    ///
    /// The output is formatted in tabular form. Column headers are included that help
    /// identify what data is being is being reported. The report text is human readable.
    ///
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    fn text_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> Result<()>;
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
    fn json_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter, pretty: bool) -> Result<()>;
    /// Generate a report in CSV format.
    ///
    /// The CSV data is separated by the ***,*** character. At some point a command line argument
    /// may be added to control this behavior.
    /// # Arguments
    ///
    /// * `weather_data` - The domain API used to access weather data.
    /// * `report_writer` - The output manager that controls where report output will be sent.
    fn csv_output(&self, weather_data: &WeatherData, report_writer: &ReportWriter) -> Result<()>;
}

/// Proprties that are common to commands that produce reports.
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
    /// The name of a file report output will be written too.
    #[clap(short = 'r', long = "report", value_name = "FILE", forbid_empty_values = true, parse(try_from_str = parse_filename))]
    pub filepath: Option<PathBuf>,
    /// Append to the log file, otherwise overwrite.
    #[clap(short = 'A', long, requires = "filepath")]
    pub append: bool,
    /// For JSON reports have content be more human readable.
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
    pub fn dispatch(&self, report_generator: &impl ReportGenerator, weather_data: &WeatherData) -> Result<()> {
        let report_writer = ReportWriter::new(&self.filepath, self.append);
        match (self.text, self.csv, self.json) {
            (false, false, false) => report_generator.text_output(weather_data, &report_writer),
            (true, false, false) => report_generator.text_output(weather_data, &report_writer),
            (false, true, false) => report_generator.csv_output(weather_data, &report_writer),
            (false, false, true) => report_generator.json_output(weather_data, &report_writer, self.pretty),
            _ => Err(Error::from("The report type is not valid...")),
        }
    }
}

/// Manages where a generated report will be written to.
///
/// If the command line includes a filename argument, generated output will be written to it
/// otherwise output will be sent to *stdout*. The writer is reusable.
pub struct ReportWriter<'a> {
    path_option: &'a Option<PathBuf>,
    append: bool,
}
impl ReportWriter<'_> {
    /// Creates an instance of the report writer container.
    ///
    /// # Arguments
    ///
    /// * `pathbuf_option` - the optional file pathname where generated reports will be written.
    ///
    pub fn new(path_option: &Option<PathBuf>, append: bool) -> ReportWriter {
        ReportWriter { path_option, append }
    }
    /// Creates `Write` instance where reports can be written.
    ///
    /// If the report writer contains a file pathname, an error can occur due to permission
    /// or locking issues.
    pub fn get_writer(&self) -> Result<Box<dyn io::Write>> {
        let writer = get_writer(&self.path_option, self.append)?;
        Ok(writer)
    }
    /// Creates the report.
    ///
    /// The report output is written to the filename provided on the command line
    /// or to `stdout`.
    pub fn create(&self, report: &Report) -> Result<()> {
        write_strings(&mut self.get_writer()?, report.into_iter())?;
        Ok(())
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
fn parse_filename(filename: &str) -> result::Result<PathBuf, String> {
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
pub fn dispatch(cli: Cli, weather_api: &WeatherData) -> Result<()> {
    match cli.command {
        Some(Commands::ListLocations { args, report_type }) => {
            let list_locations = list_locations::ListLocations::new(args);
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
        _ => {
            println!("Try -h for command help...");
            Ok(())
        }
    }
}
