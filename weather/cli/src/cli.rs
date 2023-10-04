//! # The weather command line interface.
//!
//! The CLI is built using `clap`. Originally I wrote it using structs with `#[derive]`
//! attributes. After using the `clap` programming API in the administration tool I
//! decided to ditch the struct with attributes implementation. It took about a day to
//! make the change and I was surprised to see how much crap was removed.
//!
//! I'm genrally pleased moving to a more functional implementation. There are patterns
//! that could probably be moved to macros however I'll put up with some code duplication
//! for right now. I'm also pleased with the model surrounding command arguments and
//! mining data for the implementation.
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use std::{fmt::Display, io, path::PathBuf, result};
use toolslib::text;

mod list_history;
mod list_locations;
mod list_summary;
mod report_history;

/// The command line interface result.
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
impl From<weather_lib::Error> for Error {
    fn from(error: weather_lib::Error) -> Self {
        Error(error.to_string())
    }
}
impl From<toolslib::Error> for Error {
    fn from(error: toolslib::Error) -> Self {
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
impl From<text::Error> for Error {
    fn from(error: text::Error) -> Self {
        Error(error.to_string())
    }
}

pub use v2::{get, initialize, initialize_and_run, run, CommandArgs};
pub(self) use v2::{get_writer, ListHistory, ListLocations, ListSummary, ReportHistory};
mod v2 {
    //! The command line parsing implementation.
    use super::*;
    use chrono::NaiveDate;
    use toolslib::{logs, text};
    use weather_lib::prelude::{WeatherData, DataCriteria};
    use weather_lib::{archive_weather_data, db_weather_data, prelude::DateRange};

    /// The command line definition.
    pub fn get() -> Command {
        let binary_name = env!("CARGO_BIN_NAME");
        let version = env!("CARGO_PKG_VERSION");
        Command::new(binary_name)
            // boiler plate
            .about("The weather data command line.")
            .version(version)
            .subcommand_required(true)
            .allow_external_subcommands(false)
            // show help if nothing is on the command line
            .arg_required_else_help(true)
            // the command arguments
            .args(CommandArgs::get())
            // the subcommands
            .subcommand(ListLocations::get())
            .subcommand(ListSummary::get())
            .subcommand(ListHistory::get())
            .subcommand(ReportHistory::get())
            .subcommand(AddHistory::get())
    }

    /// This is a mainline helper that prepares the runtime environment and runs the command.
    ///
    /// # Arguments
    ///
    /// * `args` holds the arguments from the parsed command line.
    pub fn initialize_and_run(args: ArgMatches) -> Result<()> {
        let cmd_args = CommandArgs::from(&args);
        initialize(&cmd_args);
        let weather_dir = cmd_args.weather_dir();
        let weather_data =
            if cmd_args.db() { db_weather_data(weather_dir)? } else { archive_weather_data(weather_dir)? };

        run(&weather_data, args)
    }

    /// Prepare the runtime environment
    ///
    /// # Arguments
    ///
    /// * `args` holds the arguments from the parsed command line.
    pub fn initialize(cmd_args: &CommandArgs) {
        match logs::initialize(logs::LogProperties {
            level: match cmd_args.verbosity() {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
            console_pattern: None,
            logfile_pattern: None,
            logfile_path: cmd_args.logfile(),
            logfile_append: cmd_args.append(),
            file_loggers: vec![String::from("toolslib"), String::from("weather"), String::from("weather_lib")],
        }) {
            Ok(_) => (),
            Err(log_error) => eprintln!("Error initializing logging!!! {:?}", log_error),
        };
    }

    /// Run the appropriate subcommand.
    ///
    /// # Arguments
    ///
    /// * `weather_data` is the weather library API used by the subcommands.
    /// * `args` holds the arguments from the parsed command line.
    pub fn run(api: &WeatherData, mut args: ArgMatches) -> Result<()> {
        if let Some((name, cmd_args)) = args.remove_subcommand() {
            match name.as_str() {
                ListLocations::NAME => ListLocations::run(api, cmd_args),
                ListSummary::NAME => ListSummary::run(api, cmd_args),
                ListHistory::NAME => ListHistory::run(api, cmd_args),
                ReportHistory::NAME => ReportHistory::run(api, cmd_args),
                AddHistory::NAME => AddHistory::run(api, cmd_args),
                _ => unreachable!("A subcommand was not dispatch..."),
            }
        } else {
            unreachable!("There was no subcommand available...")
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
    fn parse_filename(filename: &str) -> std::result::Result<PathBuf, String> {
        if filename.is_empty() {
            Err(format!("The filename cannot be empty."))
        } else {
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
    }

    /// Creates a `Write` instance where reports will be written.
    ///
    /// If the report writer contains a file pathname, an error can occur due to permission
    /// or locking issues.
    ///
    /// # Arguments
    ///
    /// * `report_args` has the command line arguments surrounding report generation.
    pub(in crate::cli) fn get_writer(report_args: &ReportArgs) -> Result<Box<dyn io::Write>> {
        let writer = text::get_writer(&report_args.report_file(), report_args.append())?;
        Ok(writer)
    }

    /// The CLI list history subcommand definition.
    #[derive(Debug)]
    pub(in crate::cli) struct ListHistory(
        /// The list history command arguments.
        ArgMatches,
    );
    impl ListHistory {
        /// The list history command name.
        const NAME: &str = "lh";
        /// Create the list history command.
        fn get() -> Command {
            Command::new(Self::NAME)
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
        /// * `args` contains the list history command arguments.
        fn run(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
            list_history::execute(weather_data, Self(args))
        }
        /// Get the report arguments.
        pub(in crate::cli) fn report_args(&self) -> ReportArgs {
            ReportArgs(&self.0)
        }
        /// Get the location criteria.
        pub(in crate::cli) fn criteria_args(&self) -> CriteriaArgs {
            CriteriaArgs(&self.0)
        }
    }

    /// The CLI list locations subcommand definition.
    #[derive(Debug)]
    pub(in crate::cli) struct ListLocations(
        /// The list locations command arguments.
        ArgMatches,
    );
    impl ListLocations {
        /// The list locations command name.
        const NAME: &str = "ll";
        /// Create the list locations command.
        fn get() -> Command {
            Command::new(Self::NAME)
                .about("List the weather data locations that are available.")
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
        fn run(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
            list_locations::execute(weather_data, Self(args))
        }
        /// Get the report arguments.
        pub(in crate::cli) fn report_args(&self) -> ReportArgs {
            ReportArgs(&self.0)
        }
        /// Get the location criteria.
        pub(in crate::cli) fn criteria_args(&self) -> CriteriaArgs {
            CriteriaArgs(&self.0)
        }
    }

    /// The CLI list summary subcommand definition.
    #[derive(Debug)]
    pub(in crate::cli) struct ListSummary(
        /// The list summary command arguments.
        ArgMatches,
    );
    impl ListSummary {
        /// The list summary command name.
        const NAME: &str = "ls";
        /// create the list summary command.
        fn get() -> Command {
            Command::new(Self::NAME)
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
        /// * `args` contains the list history command arguments.
        fn run(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
            list_summary::execute(weather_data, Self(args))
        }
        /// Get the report arguments.
        pub(in crate::cli) fn report_args(&self) -> ReportArgs {
            ReportArgs(&self.0)
        }
        /// Get the location criteria.
        pub(in crate::cli) fn criteria_args(&self) -> CriteriaArgs {
            CriteriaArgs(&self.0)
        }
    }

    /// The CLI report history subcommand definition.
    #[derive(Debug)]
    pub(in crate::cli) struct ReportHistory(
        /// The report history command arguments.
        ArgMatches,
    );
    impl ReportHistory {
        /// The report history command name.
        const NAME: &str = "rh";
        /// The report temperature argument id.
        const TEMPS: &str = "HISTORY_TEMPS";
        /// The report conditions argument id.
        const CONDITIONS: &str = "HISTORY_COND";
        /// The report precipitation argument id.
        const PRECIPITATION: &str = "HISTORY_PERCIP";
        /// The report summary argument id.
        const SUMMARY: &str = "HISTORY_SUM";
        /// The report all argument id.
        const ALL: &str = "ALL_HISTORIES";
        /// The location argument id.
        const LOCATION: &str = "HISTORY_LOCATION";
        /// The history from date argument id.
        const FROM: &str = "HISTORY_FROM";
        /// The history thru date argument id.
        const THRU: &str = "HISTORY_THRU";
        /// Create the report history command.
        fn get() -> Command {
            let cmd_args = [
                Arg::new(Self::TEMPS)
                    .short('t')
                    .long("temp")
                    .action(ArgAction::SetTrue)
                    .conflicts_with(Self::ALL)
                    .help("Include temperature information in the report (default)."),
                Arg::new(Self::PRECIPITATION)
                    .short('p')
                    .long("precip")
                    .action(ArgAction::SetTrue)
                    .conflicts_with(Self::ALL)
                    .help("Include percipitation information in the report."),
                Arg::new(Self::CONDITIONS)
                    .short('c')
                    .long("cnd")
                    .action(ArgAction::SetTrue)
                    .conflicts_with(Self::ALL)
                    .help("Include weather conditions in the report."),
                Arg::new(Self::SUMMARY)
                    .short('s')
                    .long("sum")
                    .action(ArgAction::SetTrue)
                    .conflicts_with(Self::ALL)
                    .help("Include summary information in the report."),
                Arg::new(Self::ALL)
                    .short('a')
                    .long("all")
                    .action(ArgAction::SetTrue)
                    .help("Include all weather information in the report."),
                Arg::new(Self::LOCATION)
                    .action(ArgAction::Set)
                    .required(true)
                    .value_name("LOCATION")
                    .value_parser(location_parser)
                    .help("The location to use for the weather history."),
                Arg::new(Self::FROM)
                    .action(ArgAction::Set)
                    .required(true)
                    .value_parser(date_parser)
                    .value_name("FROM")
                    .help("The weather history starting date."),
                Arg::new(Self::THRU)
                    .action(ArgAction::Set)
                    .required(false)
                    .value_parser(date_parser)
                    .value_name("THRU")
                    .help("The weather history ending date."),
            ];
            Command::new(Self::NAME)
                .about("Generate a weather history report for a location.")
                .args(cmd_args)
                .args(ReportArgs::get())
                .group(ReportArgs::arg_group())
                .arg_required_else_help(true)
        }
        /// Executes the report history command.
        ///
        /// # Arguments
        ///
        /// * `weather_data` is the weather library API used by the command.
        /// * `args` contains the report history command arguments.
        fn run(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
            report_history::execute(weather_data, Self(args))
        }
        /// Get the report temperatures flag.
        ///
        /// It will be `true` if there are no other report information flags set.
        pub(in crate::cli) fn temps(&self) -> bool {
            let mut set = self.0.get_flag(Self::TEMPS) || self.all();
            if !set {
                // if none of the other switches are set force temps true
                set = !(self.conditions() || self.precipitation() || self.summary())
            }
            set
        }
        /// Get the report conditions flag.
        pub(in crate::cli) fn conditions(&self) -> bool {
            self.0.get_flag(Self::CONDITIONS) || self.all()
        }
        /// Get the report precipitation flag.
        pub(in crate::cli) fn precipitation(&self) -> bool {
            self.0.get_flag(Self::PRECIPITATION) || self.all()
        }
        /// Get the report summary flag.
        pub(in crate::cli) fn summary(&self) -> bool {
            self.0.get_flag(Self::SUMMARY) || self.all()
        }
        /// Get the report all flag.
        pub(in crate::cli) fn all(&self) -> bool {
            self.0.get_flag(Self::ALL)
        }
        /// Get the report location argument.
        pub(in crate::cli) fn location(&self) -> String {
            self.0.get_one::<String>(Self::LOCATION).unwrap().clone()
        }
        /// Get the report starting date argument.
        pub(in crate::cli) fn from(&self) -> &NaiveDate {
            self.0.get_one::<NaiveDate>(Self::FROM).unwrap()
        }
        /// Get the report ending date argument.
        pub(in crate::cli) fn thru(&self) -> Option<&NaiveDate> {
            self.0.get_one::<NaiveDate>(Self::THRU)
        }
        /// Get the date arguments as a [DateRange].
        pub(in crate::cli) fn date_range(&self) -> DateRange {
            let from = self.from();
            let to = self.thru().map_or(from, |d| d);
            DateRange { from: from.clone(), to: to.clone() }
        }
        /// Get the report arguments.
        pub(in crate::cli) fn report_args(&self) -> ReportArgs {
            ReportArgs(&self.0)
        }
    }

    /// Parse the location argument making sure it is not a date.
    ///
    /// # Arguments
    ///
    /// * `location` is the argument that will be parsed.
    fn location_parser(location: &str) -> std::result::Result<String, String> {
        if date_parser(location).is_err() {
            Ok(location.to_string())
        } else {
            Err("A location was not provided.".to_string())
        }
    }

    /// Parse an argument turning it into a [NaiveDate].
    ///
    /// # Arguments
    ///
    /// * `date` is the argument that will be parsed.
    fn date_parser(date: &str) -> std::result::Result<NaiveDate, String> {
        match toolslib::date_time::parse_date(date) {
            Ok(date) => Ok(date),
            Err(err) => Err(err.to_string()),
        }
    }

    /// The CLI add history subcommand definition.
    pub(in crate::cli) struct AddHistory(
        /// The add history command arguments.
        ArgMatches,
    );
    impl AddHistory {
        /// The list summary command name.
        const NAME: &str = "ah";
        /// The location argument id.
        const LOCATION: &str = "HISTORY_LOCATION";
        /// The history from date argument id.
        const FROM: &str = "HISTORY_FROM";
        /// The history thru date argument id.
        const THRU: &str = "HISTORY_THRU";
        /// create the list summary command.
        fn get() -> Command {
            Command::new(Self::NAME)
                .about("Add weather history to a location.")
                .arg(
                    Arg::new(Self::LOCATION)
                        .action(ArgAction::Set)
                        .required(true)
                        .value_name("LOCATION")
                        .value_parser(location_parser)
                        .help("The location to use for the weather history."),
                )
                .arg(
                    Arg::new(Self::FROM)
                        .action(ArgAction::Set)
                        .required(true)
                        .value_parser(date_parser)
                        .value_name("FROM")
                        .help("The weather history starting date."),
                )
                .arg(
                    Arg::new(Self::THRU)
                        .action(ArgAction::Set)
                        .required(false)
                        .value_parser(date_parser)
                        .value_name("THRU")
                        .help("The weather history ending date."),
                )
                .arg_required_else_help(true)
        }
        /// Executes the report history command.
        ///
        /// # Arguments
        ///
        /// * `weather_data` is the weather library API used by the command.
        /// * `args` contains the report history command arguments.
        fn run(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
            let location = args.get_one::<String>(Self::LOCATION).unwrap().clone();
            let criteria = DataCriteria{ filters: vec!(location), icase: true, sort: false };
            let from = args.get_one::<NaiveDate>(Self::FROM).unwrap();
            let to = args.get_one::<NaiveDate>(Self::THRU).map_or(from, |d| d);
            let date_range = DateRange { from: from.clone(), to: to.clone() };
            let additions = weather_data.add_history(criteria, date_range)?;
            println!("{} histories were added.", additions);
            Ok(())
        }
    }

    /// The common command line arguments.
    pub struct CommandArgs<'a>(
        /// The subcommand command line arguments.
        &'a ArgMatches,
    );
    impl<'a> CommandArgs<'a> {
        /// The weather directory argument id.
        const WEATHER_DIR: &str = "WEATHER_DIR";
        /// The log file argument id.
        const LOGFILE: &str = "LOGFILE";
        /// The append to log file argument id.
        const APPEND: &str = "APPEND_LOGFILE";
        /// The logging verbosity level argument id..
        const VERBOSITY: &str = "LOG_VERBOSITY";
        /// Use a database configuration for weather history.
        const DB: &str = "USE_DB";
        fn get() -> Vec<Arg> {
            vec![
                Arg::new(Self::WEATHER_DIR)
                    .short('d')
                    .long("directory")
                    .action(ArgAction::Set)
                    .value_parser(Self::parse_weather_dir)
                    .value_name("DIR")
                    .help("The weather data directory pathname."),
                Arg::new(Self::DB)
                    .long("db")
                    .action(ArgAction::SetTrue)
                    .help("Use a database configuration for weather history."),
                Arg::new(Self::LOGFILE)
                    .short('l')
                    .long("logfile")
                    .action(ArgAction::Set)
                    .value_parser(parse_filename)
                    .help("The log filename (DEFAULT stdout)."),
                Arg::new(Self::APPEND)
                    .short('a')
                    .long("append")
                    .requires(Self::LOGFILE)
                    .action(ArgAction::SetTrue)
                    .help("Append to the logfile, otherwise overwrite."),
                Arg::new(Self::VERBOSITY)
                    .short('v')
                    .long("verbose")
                    .action(ArgAction::Count)
                    .help("Logging verbosity (once=INFO, twice=DEBUG, +twice=TRACE)"),
            ]
        }
        /// Parse the weather directory argument.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the weather directory command argument.
        fn parse_weather_dir(dirname: &str) -> std::result::Result<String, String> {
            let path = PathBuf::from(dirname);
            if path.is_dir() {
                Ok(dirname.to_string())
            } else if path.exists() {
                Err(format!("{} is not a directory.", dirname))
            } else {
                Err(format!("{} does not exist.", dirname))
            }
        }
        /// Get the weather directory argument.
        pub fn weather_dir(&self) -> &str {
            self.0.get_one::<String>(Self::WEATHER_DIR).map_or(Default::default(), |s| s.as_str())
        }
        /// Get the logfile name argument.
        pub fn logfile(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(Self::LOGFILE).map_or(Default::default(), |p| Some(p.clone()))
        }
        /// Get the append to logfile flag.
        pub fn append(&self) -> bool {
            self.0.get_flag(Self::APPEND)
        }
        /// Get the use a database configuration flag.
        pub fn db(&self) -> bool {
            self.0.get_flag(Self::DB)
        }
        /// Get the logging verbosity flag.
        pub fn verbosity(&self) -> u8 {
            std::cmp::min(self.0.get_one::<u8>(Self::VERBOSITY).map_or(0, |a| *a), 3)
        }
    }
    impl<'a> From<&'a ArgMatches> for CommandArgs<'a> {
        fn from(args: &'a ArgMatches) -> Self {
            Self(args)
        }
    }

    /// The common command line reporting options.
    pub(in crate::cli) struct ReportArgs<'a>(
        /// The subcommand command line arguments.
        &'a ArgMatches,
    );
    impl<'a> ReportArgs<'a> {
        /// Generate a textual based report.
        const TEXT: &str = "REPORT_TEXT";
        /// Generate a CSV based report.
        const CSV: &str = "REPORT_CSV";
        /// Generate a JSON based report.
        const JSON: &str = "REPORT_JSON";
        /// For JSON reports output the data in a pretty format.
        const PRETTY: &str = "REPORT_JSON_PRETTY";
        /// The name of the report file.
        const REPORT_FILE: &str = "REPORT_FILE";
        /// Append data to the report file.
        const APPEND: &str = "REPORT_APPEND";
        /// Get the report arguments.
        fn get() -> Vec<Arg> {
            vec![
                Arg::new(Self::TEXT)
                    .long("text")
                    .action(ArgAction::SetTrue)
                    .help("The report will be plain Text (default)"),
                Arg::new(Self::CSV).long("csv").action(ArgAction::SetTrue).help("The report will be in CSV format."),
                Arg::new(Self::JSON).long("json").action(ArgAction::SetTrue).help("The report will be in JSON format."),
                Arg::new(Self::PRETTY)
                    .short('P')
                    .long("pretty")
                    .action(ArgAction::SetTrue)
                    // it seems flags are always present in the parsed arg list and I can't find a way to require it
                    // only when JSON is true.
                    .requires(Self::JSON)
                    .help("For JSON reports output will be pretty printed."),
                Arg::new(Self::REPORT_FILE)
                    .short('r')
                    .long("report")
                    .value_name("FILE")
                    .action(ArgAction::Set)
                    .value_parser(parse_filename)
                    .help("The report filename (default stdout)."),
                Arg::new(Self::APPEND)
                    .short('A')
                    .long("append")
                    .requires(Self::REPORT_FILE)
                    .action(ArgAction::SetTrue)
                    .help("Append to the report file, otherwise overwrite."),
            ]
        }
        /// Get the command argument group for selecting either text, CSV, or JSON reports,
        fn arg_group() -> ArgGroup {
            ArgGroup::new("REPORT_TYPES").args([Self::TEXT, Self::CSV, Self::JSON]).required(false)
        }
        /// Get the text based report flag.
        #[allow(unused)]
        pub(in crate::cli) fn text(&self) -> bool {
            self.0.get_flag(ReportArgs::TEXT) || !(self.csv() || self.json())
        }
        /// Get the `CSV` based report flag.
        pub(in crate::cli) fn csv(&self) -> bool {
            self.0.get_flag(ReportArgs::CSV)
        }
        /// Get the `JSON` based report flag.
        pub(in crate::cli) fn json(&self) -> bool {
            self.0.get_flag(ReportArgs::JSON)
        }
        /// Get the `JSON` pretty printed report flag.
        pub(in crate::cli) fn pretty(&self) -> bool {
            self.0.get_flag(ReportArgs::PRETTY)
        }
        /// Get the append to report flag.
        pub(in crate::cli) fn append(&self) -> bool {
            self.0.get_flag(ReportArgs::APPEND)
        }
        /// Get the report filename argument.
        pub(in crate::cli) fn report_file(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(ReportArgs::REPORT_FILE).map_or(None, |p| Some(p.clone()))
        }
    }

    /// The common command locations criteria.
    pub(in crate::cli) struct CriteriaArgs<'a>(
        /// The subcommand command line arguments.
        &'a ArgMatches,
    );
    impl<'a> CriteriaArgs<'a> {
        /// The locations filter.
        const LOCATIONS: &str = "LOCATIONS_FILTER";
        /// Get the criteria arguments.
        fn get() -> Vec<Arg> {
            vec![Arg::new(Self::LOCATIONS)
                .action(ArgAction::Append)
                .help("Filter output to these locations (Optional)")]
        }
        /// Get the collection of location names.
        pub(in crate::cli) fn locations(&self) -> Vec<String> {
            match self.0.get_many::<String>(Self::LOCATIONS) {
                Some(filters) => filters.map(|f| f.clone()).collect(),
                None => vec![],
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn cli() {
            // bootstrap up the cli to make sure there aren't subcommand argument collisions.
            match get().no_binary_name(true).try_get_matches_from(vec!["--version"]) {
                Ok(_) => unreachable!("clap should return an error for version"),
                Err(err) => assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion),
            };
        }

        #[test]
        fn criteria() {
            fn testcase(args: &[&str]) -> ArgMatches {
                let cmd = Command::new("test")
                    .no_binary_name(true)
                    .subcommand(Command::new("testcase").args(CriteriaArgs::get()));
                let mut raw_args = cmd.try_get_matches_from(args).unwrap();
                let (_, args) = raw_args.remove_subcommand().unwrap();
                args
            }
            let args = testcase(&["testcase"]);
            assert_eq!(CriteriaArgs(&args).locations().len(), 0);
            let args = testcase(&["testcase", "foo", "bar"]);
            let locations = CriteriaArgs(&args).locations();
            assert_eq!(CriteriaArgs(&args).locations().len(), 2);
            assert!(locations.iter().any(|s| s == "foo"));
            assert!(locations.iter().any(|s| s == "bar"));
        }

        #[test]
        fn report_args() {
            let mut cmd = Command::new("test")
                .no_binary_name(true)
                .subcommand(Command::new("testcase").args(ReportArgs::get()).group(ReportArgs::arg_group()));
            fn testcase(cmd: &mut Command, args: &[&str]) -> ArgMatches {
                let mut raw_args = cmd.try_get_matches_from_mut(args).unwrap();
                let (_, args) = raw_args.remove_subcommand().unwrap();
                args
            }
            // let args = testcase(&["testcase"]);
            let cmd_args = testcase(&mut cmd, &["testcase"]);
            let report_args = ReportArgs(&cmd_args);
            assert!(report_args.text());
            assert!(!report_args.csv());
            assert!(!report_args.json());
            assert!(!report_args.append());
            assert_eq!(report_args.report_file(), None);
            let cmd_args = testcase(&mut cmd, &["testcase", "--report", "foobar.rpt", "--append"]);
            let report_args = ReportArgs(&cmd_args);
            assert!(report_args.text());
            assert!(!report_args.csv());
            assert!(!report_args.json());
            assert!(report_args.append());
            assert_eq!(report_args.report_file().unwrap(), PathBuf::from("foobar.rpt"));
            let args = testcase(&mut cmd, &["testcase", "--csv"]);
            let report_args = ReportArgs(&args);
            assert!(!report_args.text());
            assert!(report_args.csv());
            assert!(!report_args.json());
            let args = testcase(&mut cmd, &["testcase", "--json"]);
            let report_args = ReportArgs(&args);
            assert!(!report_args.text());
            assert!(!report_args.csv());
            assert!(report_args.json());
            assert!(!report_args.pretty());
            let args = testcase(&mut cmd, &["testcase", "--json", "--pretty"]);
            let report_args = ReportArgs(&args);
            assert!(!report_args.text());
            assert!(!report_args.csv());
            assert!(report_args.json());
            assert!(report_args.pretty());
            assert!(cmd.try_get_matches_from_mut(["testcase", "--text", "--csv"]).is_err());
            assert!(cmd.try_get_matches_from_mut(["testcase", "--text", "--json"]).is_err());
            assert!(cmd.try_get_matches_from_mut(["testcase", "--csv", "--json"]).is_err());
        }

        #[test]
        fn command_args() {
            let mut cmd =
                Command::new("test").no_binary_name(true).subcommand(Command::new("testcase").args(CommandArgs::get()));
            fn arg_matches(cmd: &mut Command, args: &[&str]) -> ArgMatches {
                let mut raw_args = cmd.try_get_matches_from_mut(args).unwrap();
                let (_, args) = raw_args.remove_subcommand().unwrap();
                args
            }
            let matches = arg_matches(&mut cmd, &["testcase"]);
            let command_args = CommandArgs(&matches);
            assert!(command_args.weather_dir().is_empty());
            assert!(command_args.logfile().is_none());
            assert!(!command_args.append());
            assert!(!command_args.db());
            assert_eq!(command_args.verbosity(), 0);
            let known_dir = env!("CARGO_MANIFEST_DIR");
            let matches = arg_matches(&mut cmd, &["testcase", "-d", known_dir, "-l", "logfile", "-a", "-vvvv", "--db"]);
            let command_args = CommandArgs(&matches);
            assert_eq!(command_args.weather_dir(), known_dir);
            assert_eq!(command_args.logfile().unwrap(), PathBuf::from("logfile"));
            assert!(command_args.append());
            assert!(command_args.db());
            assert_eq!(command_args.verbosity(), 3)
        }
    }
}
