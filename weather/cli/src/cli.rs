//! # The weather command line interface.
//!
//! The CLI is built using `clap`. Originally I wrote it using structs with `#[derive]`
//! attributes. After using the `clap` programming API in the administration tool I
//! decided to ditch the attribute implementation. It took about a day to
//! make the change. I was surprised to see how much crap was removed.
//!
//! I'm generally pleased moving to a more functional implementation. There are patterns
//! that could probably be moved to macros however I'll put up with some code duplication
//! for right now. I'm also pleased with the model surrounding command arguments and
//! mining data for the implementation.

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use std::{io, path::PathBuf};

mod admin;
mod reports;
mod tui;
mod user;

/// The command line interface result.
pub type Result<T> = std::result::Result<T, Error>;

/// The CLI error definition.
#[derive(Debug)]
pub struct Error(String);
impl std::fmt::Display for Error {
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
impl From<toolslib::Error> for Error {
    fn from(error: toolslib::Error) -> Self {
        Error(error.to_string())
    }
}
impl From<toolslib::text::Error> for Error {
    fn from(error: toolslib::text::Error) -> Self {
        Error(error.to_string())
    }
}
impl From<termui_lib::Error> for Error {
    fn from(error: termui_lib::Error) -> Self {
        Error(error.to_string())
    }
}

// pub use current::{get, initialize, initialize_and_run, run, CommandLineArgs};
pub use current::{get, initialize_and_run};
use current::{get_writer, CriteriaArgs, ReportArgs};
mod current {
    //! The current command line implementation.
    use toolslib::logs;

    use admin::Admin;
    use user::User;
    use weather_lib::{admin_prelude::weather_admin, prelude::WeatherData, create_weather_data};

    use super::*;

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
            .args(CommandLineArgs::get())
            // the subcommands
            .subcommands(User::get_commands())
            .subcommand(TerminalUI::get())
            .subcommand(Admin::get())
    }

    /// This is a mainline helper that prepares the runtime environment and runs the command.
    ///
    /// # Arguments
    ///
    /// * `args` holds the arguments from the parsed command line.
    pub fn initialize_and_run(args: ArgMatches) -> Result<()> {
        initialize(&args);
        log::trace!("initialize_and_run Enter");
        run(args)
    }

    /// Prepare the runtime environment
    ///
    /// # Arguments
    ///
    /// * `args` holds the arguments from the parsed command line.
    pub fn initialize(args: &ArgMatches) {
        let cmd_args = CommandLineArgs::from(args);
        let (logfile, append) = match args.subcommand_name().unwrap_or("") == TerminalUI::NAME {
            true => match cmd_args.logfile() {
                Some(logfile) => (Some(logfile), cmd_args.append()),
                None => (Some(PathBuf::from("weather.log")), false),
            },
            false => (cmd_args.logfile(), cmd_args.append()),
        };
        match logs::initialize(logs::LogProperties {
            level: match cmd_args.verbosity() {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
            console_pattern: None,
            logfile_pattern: None,
            logfile_path: logfile,
            logfile_append: append,
            file_loggers: vec![
                "cli".to_string(),
                "toolslib".to_string(),
                "weather".to_string(),
                "weather_lib".to_string(),
                "termui_lib".to_string(),
            ],
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
    pub fn run(mut args: ArgMatches) -> Result<()> {
        let (name, subcommand_args) = args.remove_subcommand().expect("CLI command not found...");
        let command_args = CommandLineArgs::from(&args);
        match name.as_str() {
            Admin::NAME => run_admin(command_args, subcommand_args),
            _ => run_user(&name, command_args, subcommand_args),
        }
    }

    fn run_admin(command_args: CommandLineArgs, args: ArgMatches) -> Result<()> {
        let weather_dir = command_args.weather_dir();
        let weather_admin = weather_admin(weather_dir)?;
        Admin::run(&weather_admin, args)
    }

    fn run_user(name: &str, command_args: CommandLineArgs, args: ArgMatches) -> Result<()> {
        let config_file = command_args.config_file();
        let weather_dir = command_args.weather_dir();
        let no_db = command_args.no_db();
        let weather_data = create_weather_data(config_file, weather_dir, no_db)?;
        match name {
            // TerminalUI::NAME => TerminalUI::run_tui(&weather_data, args),
            TerminalUI::NAME => TerminalUI::run_tui(weather_data, args),
            _ => User::run(&weather_data, name, args),
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
    pub fn parse_filename(filename: &str) -> std::result::Result<PathBuf, String> {
        if filename.is_empty() {
            Err("The filename cannot be empty.".to_string())
        } else {
            let filepath = PathBuf::from(filename);
            if filepath.is_dir() {
                Err(format!("{} is a directory...", filename))
            } else if filepath.is_symlink() {
                Err(format!("{} is a symlink...", filename))
            } else if filepath.is_absolute() && !filepath.parent().unwrap().exists() {
                Err("The parent directory does not exist...".to_string())
            } else {
                // you can read all about this but "bar.txt" and "foo/bar.txt" are both relative AND
                // have parent paths, one just happens to be empty...
                let parent = filepath.parent().unwrap();
                if parent.to_str().unwrap().len() > 0 && !parent.exists() {
                    Err("The relative path to file does not exist...".to_string())
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
    pub fn get_writer(report_args: &ReportArgs) -> Result<Box<dyn io::Write>> {
        let writer = toolslib::text::get_writer(&report_args.report_file(), report_args.append())?;
        Ok(writer)
    }

    #[derive(Debug)]
    struct TerminalUI;
    impl TerminalUI {
        /// The command name.
        const NAME: &'static str = "tui";
        /// Create the sub-command.
        fn get() -> Command {
            Command::new(Self::NAME).about("A Terminal based weather data UI.")
        }
        /// Executes the command.
        ///
        /// # Arguments
        ///
        /// * `weather_data` is the weather data API.
        /// * `args` contains the report history command arguments.
        fn run_tui(weather_data: WeatherData, _args: ArgMatches) -> Result<()> {
            // let cmd_args = Self(args);
            tui::weather_ui(weather_data)?;
            Ok(())
        }
    }

    /// The common command line arguments.
    pub struct CommandLineArgs<'a>(
        /// The subcommand command line arguments.
        &'a ArgMatches,
    );
    impl<'a> CommandLineArgs<'a> {
        /// The config file argument id.
        const CONFIG_FILE: &'static str = "CONFIG_FILE";
        /// The weather directory argument id.
        const WEATHER_DIR: &'static str = "WEATHER_DIR";
        /// The log file argument id.
        const LOGFILE: &'static str = "LOGFILE";
        /// The append to log file argument id.
        const APPEND: &'static str = "APPEND_LOGFILE";
        /// The logging verbosity level argument id.
        const VERBOSITY: &'static str = "LOG_VERBOSITY";
        /// Use the filesystem implementation of weather data.
        const FS: &'static str = "FS";
        /// Get the common command line arguments.
        fn get() -> Vec<Arg> {
            vec![
                Arg::new(Self::CONFIG_FILE)
                    .short('c')
                    .long("config")
                    .action(ArgAction::Set)
                    .value_name("FILE")
                    // .require_equals(true)
                    .value_parser(parse_filename)
                    .help("The configuration file pathname (DEFAULT weather.toml)."),
                Arg::new(Self::WEATHER_DIR)
                    .short('d')
                    .long("directory")
                    .action(ArgAction::Set)
                    .value_name("DIR")
                    // .require_equals(true)
                    .value_parser(Self::parse_weather_dir)
                    .help("The weather data directory pathname."),
                Arg::new(Self::FS)
                    .long("fs")
                    .action(ArgAction::SetTrue)
                    .help("Do not use a weather history DB if one is available."),
                Arg::new(Self::LOGFILE)
                    .short('l')
                    .long("logfile")
                    .action(ArgAction::Set)
                    .value_name("FILE")
                    // .require_equals(true)
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
        fn parse_weather_dir(dirname: &str) -> std::result::Result<PathBuf, String> {
            let path = PathBuf::from(dirname);
            if path.is_dir() {
                Ok(path)
            } else if path.exists() {
                Err(format!("{} is not a directory.", dirname))
            } else {
                Err(format!("{} does not exist.", dirname))
            }
        }
        /// Get the weather directory argument.
        pub fn config_file(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(Self::CONFIG_FILE).map_or(Default::default(), |p| Some(p.clone()))
        }
        /// Get the weather directory argument.
        pub fn weather_dir(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(Self::WEATHER_DIR).map_or(Default::default(), |p| Some(p.clone()))
        }
        /// Get the logfile name argument.
        pub fn logfile(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(Self::LOGFILE).map_or(Default::default(), |p| Some(p.clone()))
        }
        /// Get the flag controlling if the logfile should be appended too.
        pub fn append(&self) -> bool {
            self.0.get_flag(Self::APPEND)
        }
        /// Get the use a database configuration flag.
        pub fn no_db(&self) -> bool {
            self.0.get_flag(Self::FS)
        }
        /// Get the logging verbosity flag.
        pub fn verbosity(&self) -> u8 {
            std::cmp::min(self.0.get_one::<u8>(Self::VERBOSITY).map_or(0, |a| *a), 3)
        }
    }
    impl<'a> From<&'a ArgMatches> for CommandLineArgs<'a> {
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
        const TEXT: &'static str = "REPORT_TEXT";
        /// Generate a CSV based report.
        const CSV: &'static str = "REPORT_CSV";
        /// Generate a JSON based report.
        const JSON: &'static str = "REPORT_JSON";
        /// For JSON reports output the data in a pretty format.
        const PRETTY: &'static str = "REPORT_JSON_PRETTY";
        /// The name of the report file.
        const REPORT_FILE: &'static str = "REPORT_FILE";
        /// Append data to the report file.
        const APPEND: &'static str = "REPORT_APPEND";
        pub fn new(args: &'a ArgMatches) -> Self {
            Self(args)
        }
        /// Get the report arguments.
        pub fn get() -> Vec<Arg> {
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
        pub fn arg_group() -> ArgGroup {
            ArgGroup::new("REPORT_TYPES").args([Self::TEXT, Self::CSV, Self::JSON]).required(false)
        }
        /// Get the text based report flag.
        #[allow(unused)]
        pub fn text(&self) -> bool {
            self.0.get_flag(ReportArgs::TEXT) || !(self.csv() || self.json())
        }
        /// Get the `CSV` based report flag.
        pub fn csv(&self) -> bool {
            self.0.get_flag(ReportArgs::CSV)
        }
        /// Get the `JSON` based report flag.
        pub fn json(&self) -> bool {
            self.0.get_flag(ReportArgs::JSON)
        }
        /// Get the `JSON` pretty printed report flag.
        pub fn pretty(&self) -> bool {
            self.0.get_flag(ReportArgs::PRETTY)
        }
        /// Get the append to report flag.
        pub fn append(&self) -> bool {
            self.0.get_flag(ReportArgs::APPEND)
        }
        /// Get the report filename argument.
        pub fn report_file(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(ReportArgs::REPORT_FILE).map_or(None, |p| Some(p.clone()))
        }
    }

    /// The common command locations_win criteria.
    pub struct CriteriaArgs<'a>(
        /// The subcommand command line arguments.
        &'a ArgMatches,
    );
    impl<'a> CriteriaArgs<'a> {
        /// The locations_win filter.
        const LOCATIONS: &'static str = "LOCATIONS_FILTER";
        pub fn new(args: &'a ArgMatches) -> Self {
            Self(args)
        }
        /// Get the criteria arguments.
        pub fn get() -> Vec<Arg> {
            vec![Arg::new(Self::LOCATIONS)
                .action(ArgAction::Append)
                .help("Filter output to these locations_win (Optional)")]
        }
        /// Get the collection of location names.
        pub fn locations(&self) -> Vec<String> {
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
            let mut cmd = Command::new("test")
                .no_binary_name(true)
                .subcommand(Command::new("testcase").args(CommandLineArgs::get()));
            macro_rules! arg_matches {
                ($cmd:expr, $args:expr) => {{
                    let mut raw_args = $cmd.try_get_matches_from_mut($args).unwrap();
                    let (_, args) = raw_args.remove_subcommand().unwrap();
                    args
                }};
            }
            let matches = arg_matches!(cmd, &["testcase"]);
            let command_args = CommandLineArgs(&matches);
            assert!(command_args.weather_dir().is_none());
            assert!(command_args.logfile().is_none());
            assert!(!command_args.append());
            assert!(!command_args.no_db());
            assert_eq!(command_args.verbosity(), 0);
            let known_dir = env!("CARGO_MANIFEST_DIR");
            let dir = format!("-d={}", known_dir);
            let matches = arg_matches!(cmd, &["testcase", dir.as_str(), "-l=logfile", "-a", "-vvvv", "--fs"]);
            let command_args = CommandLineArgs(&matches);
            assert_eq!(command_args.weather_dir().unwrap(), PathBuf::from(known_dir));
            assert_eq!(command_args.logfile().unwrap(), PathBuf::from("logfile"));
            assert!(command_args.append());
            assert!(command_args.no_db());
            assert_eq!(command_args.verbosity(), 3)
        }
    }
}
