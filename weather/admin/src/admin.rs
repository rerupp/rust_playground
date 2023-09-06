//! The weather data adminstation tool.
//!
//! When I started to plumb the API out I upgraded from an older version of `CLAP` which
//! required a fair amount of changes to the exists `derive` command lines. After going
//! thru the crate documents to resolve the issues I decided to not use `derive` this
//! time and instead use the `API` directly.
//!
//! I'm happy with the results and effort to implement the `API` with `derive`. For my
//! use with the administration `CLI` it was pretty easy to get the front end running.
//! I feel the implementation is more readable and making changes less error prone than
//! what I did with the previous implementation.
use std::{fmt::Display, result};

/// The administation result and error.
pub type Result<T> = result::Result<T, Error>;

/// The CLI error definition.
#[derive(Debug)]
pub struct Error(String);
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<weather_lib::Error> for Error {
    /// Support converting errors from the weather library.
    fn from(other: weather_lib::Error) -> Self {
        Error(other.to_string())
    }
}

/// The command line interface entry point.
fn main() -> Result<()> {
    let args = cli::get().get_matches();
    cli::initialize(&args)?;
    cli::dispatch_cmd(args)
}

use clap::{Arg, ArgAction, ArgMatches, Command};
use weather_lib::admin_prelude::*;

/// Consolidate getting boolean command line arguments.
macro_rules! bool_option {
    ($args:expr, $id:expr) => {
        $args.get_one($id).map_or(false, |m| *m)
    };
}

mod cli {
    //! The weather data `CLI` definition and implementation.
    use super::*;
    use std::path::PathBuf;
    use toolslib::logs;

    /// The weather directory argument id.
    const WEATHER_DIR: &str = "DIR";

    /// The log file argument id.
    const LOGFILE: &str = "LOG";

    /// The append to log file argument id.
    const APPEND: &str = "APPEND";

    /// The logging verbosity level argument id..
    const VERBOSITY: &str = "VERBOSE";

    /// Consolidates getting the weather directory argument.
    macro_rules! weather_dir {
        ($args:expr) => {
            $args.get_one::<String>(WEATHER_DIR).map_or(String::default(), |s| s.to_string())
        };
    }

    /// Consolidates getting the logging level argument.
    macro_rules! verbosity {
        ($args:expr) => {
            $args.get_one::<u8>(VERBOSITY).map_or(0, |v| *v)
        };
    }

    /// Consolidates getting the log filename argument.
    macro_rules! logfile {
        ($args:expr) => {
            $args.get_one::<String>(LOGFILE).map_or(None, |s| Some(PathBuf::from(s)))
        };
    }

    /// Consolidates getting the append to logfile argument.
    macro_rules! append {
        ($args:expr) => {
            bool_option!($args, APPEND)
        };
    }

    /// The command line definition.
    pub fn get() -> Command {
        let binary_name = env!("CARGO_BIN_NAME");
        let version = env!("CARGO_PKG_VERSION");
        Command::new(binary_name)
            .about("The weather data administration tool.")
            .version(version)
            .subcommand_required(true)
            .arg_required_else_help(true)
            .allow_external_subcommands(false)
            .args(cmd_args())
            .subcommand(init_cmd::get())
            .subcommand(drop_cmd::get())
            .subcommand(stat_cmd::get())
    }

    /// Run the appropriate sub-command.
    ///
    /// # Arguments
    ///
    /// * `args` contains the parsed command line arguments.
    pub fn dispatch_cmd(args: ArgMatches) -> Result<()> {
        let weather_dir = weather_dir!(args);
        let weather_admin = create_weather_admin(&weather_dir)?;
        match args.subcommand() {
            Some((init_cmd::NAME, args)) => init_cmd::dispatch(weather_admin, args),
            Some((drop_cmd::NAME, args)) => drop_cmd::dispatch(weather_admin, args),
            Some((stat_cmd::NAME, _)) => stat_cmd::dispatch(weather_admin),
            _ => unreachable!("A subcommand was not dispatch..."),
        }
    }

    /// Prepare the runtime environment
    ///
    /// # Arguments
    ///
    /// * `args` contains the parsed command line arguments.
    pub fn initialize(args: &ArgMatches) -> Result<()> {
        let log_properties = logs::LogProperties {
            level: match verbosity!(args) {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
            console_pattern: None,
            logfile_pattern: None,
            logfile_path: logfile!(args),
            logfile_append: append!(&args),
            file_loggers: vec![String::from("toolslib"), String::from("admin")],
        };
        match logs::initialize(log_properties) {
            Ok(_) => Ok(()),
            Err(err) => Err(Error(err.to_string())),
        }
    }

    /// Get the command arguments for the weather directory and log configuration.
    fn cmd_args() -> Vec<Arg> {
        vec![
            Arg::new(WEATHER_DIR)
                .short('d')
                .long("directory")
                .action(ArgAction::Set)
                .value_parser(weather_dir_parse)
                .help("The weather data directory pathname."),
            Arg::new(LOGFILE)
                .short('l')
                .long("logfile")
                .action(ArgAction::Set)
                .value_parser(logfile_parse)
                .help("The log filename (DEFAULT stdout)."),
            Arg::new(APPEND)
                .short('a')
                .long("append")
                .requires(LOGFILE)
                .action(ArgAction::SetTrue)
                .help("Append to the logfile, otherwise overwrite."),
            Arg::new(VERBOSITY)
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help("Logging verbosity (once=INFO, twice=DEBUG, +twice=TRACE)"),
        ]
    }

    /// Used by the command parser to validate the weather directory argument.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather directory command argument.
    fn weather_dir_parse(dirname: &str) -> result::Result<String, String> {
        let path = PathBuf::from(dirname);
        if path.is_dir() {
            Ok(dirname.to_string())
        } else if path.exists() {
            Err(format!("{} is not a directory.", dirname))
        } else {
            Err(format!("{} does not exist.", dirname))
        }
    }

    /// Used by the command parser to validate the log filename argument.
    ///
    /// # Arguments
    ///
    /// * `filename` is the log filename command argument.
    fn logfile_parse(filename: &str) -> result::Result<String, String> {
        let path = PathBuf::from(filename);
        if path.exists() && !path.is_file() {
            Err(format!("{} is not a writable file.", filename))
        } else {
            Ok(filename.to_string())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        /// Run the `CLI` to flush out any bootstrap problems.
        #[test]
        fn cli() {
            match super::get().try_get_matches_from(vec!["wadmin", "--version"]) {
                Ok(_) => unreachable!("clap should return an error for version"),
                Err(err) => assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion),
            };
        }

        #[test]
        fn args() {
            fn cmd() -> Command {
                super::get().subcommand(Command::new("testcase"))
            }
            // verify defaults
            let args = cmd().try_get_matches_from(["test", "testcase"]).unwrap();
            assert_eq!(logfile!(&args), None);
            assert_eq!(weather_dir!(&args), "");
            assert_eq!(append!(&args), false);
            assert_eq!(verbosity!(&args), 0);
            // verify the weather dir argument
            let known_dir = env!("CARGO_MANIFEST_DIR");
            let args = cmd().try_get_matches_from(["test", "-d", known_dir, "-vv", "testcase"]).unwrap();
            assert_eq!(logfile!(&args), None);
            assert_eq!(weather_dir!(&args), known_dir);
            assert_eq!(append!(&args), false);
            assert_eq!(verbosity!(&args), 2);
            // verify the logfile argument
            let args = cmd().try_get_matches_from(["test", "-l", "filename", "-v", "testcase"]).unwrap();
            assert_eq!(logfile!(&args), Some(PathBuf::from("filename")));
            assert_eq!(weather_dir!(&args), "");
            assert_eq!(append!(&args), false);
            assert_eq!(verbosity!(&args), 1);
            // verify the append argument
            assert!(cmd().try_get_matches_from(["test", "-a", "testcase"]).is_err());
            let args = cmd().try_get_matches_from(["test", "-l", "filename", "-a", "testcase"]).unwrap();
            assert_eq!(logfile!(&args), Some(PathBuf::from("filename")));
            assert_eq!(weather_dir!(&args), "");
            assert_eq!(append!(&args), true);
            assert_eq!(verbosity!(&args), 0);
            // verify the verbosity argument
            let args = cmd()
                .try_get_matches_from(["test", "-d", known_dir, "-l", "filename", "-a", "-vvvvvvv", "testcase"])
                .unwrap();
            assert_eq!(logfile!(&args), Some(PathBuf::from("filename")));
            assert_eq!(weather_dir!(&args), known_dir);
            assert_eq!(append!(&args), true);
            assert_eq!(verbosity!(&args), 7);
        }
    }
}

mod init_cmd {
    //! The initialize sub-command definition.
    use super::*;
    use clap::ArgGroup;

    /// The initialize sub-command name.
    pub const NAME: &str = "init";

    /// The command argument id for running in hybrid mode.
    const HYBRID: &str = "HYBRID";

    /// The command argument id for running in `JSON` document mode.
    const DOCUMENT: &str = "DOCUMENT";

    /// The command argument id for running in a full normalized mode.
    const NORMALIZE: &str = "NORMALIZE";

    /// The command argument id indicating the database schema should be dropped.
    const DROP: &str = "DROP";

    /// The command argument id indicating the database should be loaded.
    const LOAD: &str = "LOAD";

    /// The command argument id indicating `JSON` documents should be compressed in the database.
    const COMPRESS: &str = "COMPRESS";

    /// The command argument id controlling how many threads to use.
    const THREADS: &str = "THREADS";

    /// Get the initialize sub-command definition.
    pub fn get() -> Command {
        Command::new(NAME)
            .about("Initialize the weather data database.")
            .arg(
                Arg::new(HYBRID)
                    .long("hybrid")
                    .action(ArgAction::SetTrue)
                    .help("Configure the database to use archives for history data (default)."),
            )
            .arg(
                Arg::new(DOCUMENT)
                    .long("document")
                    .action(ArgAction::SetTrue)
                    .help("Configure the database to use JSON for history data."),
            )
            .arg(
                Arg::new(COMPRESS)
                    .long("compress")
                    .action(ArgAction::SetTrue)
                    .conflicts_with_all([HYBRID, NORMALIZE])
                    .requires(DOCUMENT)
                    .help("The JSON history data will be compressed in the database."),
            )
            .arg(
                Arg::new(NORMALIZE)
                    .long("normalize")
                    .action(ArgAction::SetTrue)
                    .help("Configure the database to be fully relational."),
            )
            .arg(
                Arg::new(THREADS)
                    .long("threads")
                    .action(ArgAction::Set)
                    .value_parser(thread_count_parse)
                    .default_value("1")
                    .requires(DOCUMENT)
                    .requires(NORMALIZE)
                    .conflicts_with(HYBRID)
                    .help("The number of threads to use"),
            )
            .group(ArgGroup::new("DB_MODE").args([HYBRID, DOCUMENT, NORMALIZE]).required(false))
            .arg(Arg::new(DROP).long("drop").action(ArgAction::SetTrue).help("Drops the database before initializing."))
            .arg(Arg::new(LOAD).long("load").action(ArgAction::SetTrue).help("Load the database after initializing."))
    }

    /// Collect the command line arguments and run the initialize sub-command.
    ///
    /// # Arguments
    ///
    /// * `admin_api` is the backend weather adminstration `API`.
    /// * `args` holds the initialize command arguments.
    pub fn dispatch(admin_api: WeatherAdmin, args: &ArgMatches) -> Result<()> {
        let hybrid = bool_option!(args, HYBRID);
        let document = bool_option!(args, DOCUMENT);
        let full = bool_option!(args, NORMALIZE);
        let db_config = match (hybrid, document, full) {
            (false, true, false) => {
                let compress = bool_option!(args, COMPRESS);
                DbConfig::document(compress)
            }
            (false, false, true) => DbConfig::normalize(),
            _ => DbConfig::hybrid(),
        };
        // this is safe, the thread parse already confirms it's a usize
        let threads: usize = args.get_one::<String>(THREADS).map_or(1, |m| m.parse::<usize>().unwrap());
        let drop = bool_option!(args, DROP);
        let load = bool_option!(args, LOAD);
        admin_api.init(db_config, drop, load, threads as usize)?;
        Ok(())
    }

    /// Used by the command parser to validate the thread count argument.
    ///
    /// Yeah, I know you can use a built in but the error message was bugging me.
    ///
    /// # Arguments
    ///
    /// * `dirname` is the weather directory command argument.
    fn thread_count_parse(count_arg: &str) -> result::Result<String, String> {
        match count_arg.parse::<usize>() {
            Ok(count) => {
                let max_threads = 16;
                if count <= max_threads {
                    Ok(count_arg.to_string())
                } else {
                    Err(format!("thread count is limited to {}.", max_threads))
                }
            }
            Err(_) => Err(format!("{} is not a number.", count_arg)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn args() {
            macro_rules! testcli {
                ($args:expr) => {
                    Command::new("test").subcommand(get()).try_get_matches_from($args)
                };
            }
            macro_rules! verify {
                ($args:expr, $hybrid:expr, $document:expr, $full:expr, $load:expr, $drop:expr, $compress:expr) => {
                    let testcase = $args.subcommand_matches(NAME).unwrap();
                    assert_eq!(bool_option!(testcase, HYBRID), $hybrid);
                    assert_eq!(bool_option!(testcase, DOCUMENT), $document);
                    assert_eq!(bool_option!(testcase, NORMALIZE), $full);
                    assert_eq!(bool_option!(testcase, LOAD), $load);
                    assert_eq!(bool_option!(testcase, DROP), $drop);
                    assert_eq!(bool_option!(testcase, COMPRESS), $compress);
                };
            }
            let parsed_args = testcli!(vec!["test", NAME]).unwrap();
            verify!(parsed_args, false, false, false, false, false, false);
            let parsed_args = testcli!(vec!["test", NAME, "--hybrid"]).unwrap();
            verify!(parsed_args, true, false, false, false, false, false);
            let parsed_args = testcli!(vec!["test", NAME, "--document"]).unwrap();
            verify!(parsed_args, false, true, false, false, false, false);
            let parsed_args = testcli!(vec!["test", NAME, "--document", "--compress"]).unwrap();
            verify!(parsed_args, false, true, false, false, false, true);
            assert!(testcli!(vec!["test", NAME, "--compress"]).is_err());
            let parsed_args = testcli!(vec!["test", NAME, "--normalize"]).unwrap();
            verify!(parsed_args, false, false, true, false, false, false);
            let parsed_args = testcli!(vec!["test", NAME, "--load", "--drop"]).unwrap();
            verify!(parsed_args, false, false, false, true, true, false);
        }
    }
}

mod drop_cmd {
    //! The weather data administration drop database sub-command.
    use super::*;

    /// The drop sub-command name.
    pub const NAME: &str = "drop";

    /// The command argument id to remove the existing weather data database file.
    pub const DELETE: &str = "DELETE";

    /// Get the drop sub-command definition.
    pub fn get() -> Command {
        Command::new(NAME).about("Removes the existing database schema.").arg(
            Arg::new(DELETE)
                .long("delete")
                .action(ArgAction::SetTrue)
                .help("Removes the database file from the weather data directory."),
        )
    }

    /// Collect the command line arguments and run the drop database sub-command.
    ///
    /// # Arguments
    ///
    /// * `admin_api` is the backend weather adminstration `API`.
    /// * `args` holds the drop command arguments.
    pub fn dispatch(admin_api: WeatherAdmin, args: &ArgMatches) -> Result<()> {
        let delete = bool_option!(args, DELETE);
        Ok(admin_api.drop(delete)?)
    }
}

mod stat_cmd {
    //! The weather data administration drop database sub-command.

    use super::*;

    /// The stat sub-command name.
    pub const NAME: &str = "stat";

    /// Get the drop sub-command definition.
    pub fn get() -> Command {
        Command::new(NAME).about("Get metrics about the weather data database.")
    }

    /// Collect the command line arguments and run the stat database sub-command.
    ///
    /// # Arguments
    ///
    /// * `admin_api` is the backend weather adminstation `API`.
    pub fn dispatch(admin_api: WeatherAdmin) -> Result<()> {
        let db_stat = admin_api.stat()?;
        match db_stat.config {
            Some(config) => {
                let database_mode = if config.hybrid {
                    "hybrid"
                } else if config.document {
                    match config.compress {
                        true => "document (compressed)",
                        false => "document",
                    }
                } else {
                    "normalized"
                };
                println!("Database mode: {}", database_mode);
                println!("size: {}", toolslib::mbufmt!(db_stat.size));
            }
            None => {
                println!("Weather data has not been initialized to use a database.");
            }
        }
        Ok(())
    }
}
