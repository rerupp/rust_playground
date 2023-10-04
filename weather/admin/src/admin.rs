//! The weather data adminstation tool.
//!
//! When I started to plumb the API out I upgraded from an older version of `CLAP` which
//! required a fair amount of changes to the exists `derive` command lines. After going
//! thru the crate documents to resolve the issues I decided to not use `derive` this
//! time and instead use the `API` directly.
//!
//! I'm happy with the results and effort to implement the `API` without using `derive`.
//! For my use with the administration `CLI` it was pretty easy to get the front end
//! running. I feel like the implementation is more readable and consise. Making changes
//! also seems to be less error prone than using `derive`.
use std::{
    fmt::{Debug, Display},
    process::ExitCode,
    result,
};

/// The administation result and error.
pub type Result<T> = result::Result<T, Error>;

/// The CLI error definition.
pub struct Error(String);
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}
impl Debug for Error {
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

fn main() -> ExitCode {
    let args = get_cli().get_matches();
    if let Err(error) = initialize_and_run(args) {
        eprintln!("Error: {}", error);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

pub use v2::{get as get_cli, initialize_and_run};
mod v2 {
    //! The weather data `CLI` definition and implementation.
    use super::*;
    use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
    use std::path::PathBuf;
    use toolslib::logs;
    use weather_lib::admin_prelude::*;
    use weather_lib::prelude::DataCriteria;

    /// This is a mainline helper that prepares the runtime environment and runs the command.
    ///
    /// # Arguments
    ///
    /// * `args` holds the arguments from the parsed command line.
    pub fn initialize_and_run(args: ArgMatches) -> Result<()> {
        let cmd_args = CommandArgs::from(&args);
        initialize(&cmd_args);
        let weather_dir = cmd_args.weather_dir();
        let weather_admin = create_weather_admin(&weather_dir)?;
        run(&weather_admin, args)
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

    /// Run the appropriate sub-command.
    ///
    /// # Arguments
    ///
    /// * `args` contains the parsed command line arguments.
    pub fn run(weather_admin: &WeatherAdmin, mut args: ArgMatches) -> Result<()> {
        let (name, cmd_args) = args.remove_subcommand().expect("There was no subcommand available to run");
        match (name.as_str(), cmd_args) {
            (InitCmd::NAME, cmd_args) => InitCmd::run(weather_admin, cmd_args),
            (DropCmd::NAME, cmd_args) => DropCmd::run(weather_admin, cmd_args),
            (MigrateCmd::NAME, cmd_args) => MigrateCmd::run(weather_admin, cmd_args),
            (StatCmd::NAME, cmd_args) => StatCmd::run(weather_admin, cmd_args),
            _ => unreachable!("A subcommand was not dispatch..."),
        }
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
            .args(CommandArgs::get())
            .subcommand(InitCmd::get())
            .subcommand(DropCmd::get())
            .subcommand(MigrateCmd::get())
            .subcommand(StatCmd::get())
    }

    /// The initialize command.
    #[derive(Debug)]
    struct InitCmd(
        /// The init command arguments.
        ArgMatches,
    );
    impl InitCmd {
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
        fn get() -> Command {
            Command::new(Self::NAME)
                .about("Initialize the weather data database.")
                .arg(
                    Arg::new(Self::HYBRID)
                        .long("hybrid")
                        .action(ArgAction::SetTrue)
                        .help("Configure the database to use archives for history data."),
                )
                .arg(
                    Arg::new(Self::DOCUMENT)
                        .long("document")
                        .action(ArgAction::SetTrue)
                        .help("Configure the database to use JSON for history data."),
                )
                .arg(
                    Arg::new(Self::COMPRESS)
                        .long("compress")
                        .action(ArgAction::SetTrue)
                        .conflicts_with_all([Self::HYBRID, Self::NORMALIZE])
                        .requires(Self::DOCUMENT)
                        .help("The JSON history data will be compressed in the database."),
                )
                .arg(
                    Arg::new(Self::NORMALIZE)
                        .long("normalize")
                        .action(ArgAction::SetTrue)
                        .help("Configure the database to be fully relational (default)."),
                )
                .arg(
                    Arg::new(Self::THREADS)
                        .long("threads")
                        .action(ArgAction::Set)
                        .value_parser(Self::thread_count_parse)
                        .default_value("8")
                        .requires(Self::DOCUMENT)
                        .requires(Self::NORMALIZE)
                        .conflicts_with(Self::HYBRID)
                        .help("The number of threads to use"),
                )
                .group(ArgGroup::new("DB_MODE").args([Self::HYBRID, Self::DOCUMENT, Self::NORMALIZE]).required(false))
                .arg(
                    Arg::new(Self::DROP)
                        .long("drop")
                        .action(ArgAction::SetTrue)
                        .help("Drops the database before initializing."),
                )
                .arg(
                    Arg::new(Self::LOAD)
                        .long("load")
                        .action(ArgAction::SetTrue)
                        .help("Load the database after initializing."),
                )
        }
        /// Collect the command line arguments and run the initialize sub-command.
        ///
        /// # Arguments
        ///
        /// * `admin_api` is the backend weather adminstration `API`.
        /// * `args` holds the initialize command arguments.
        fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
            let cmd_args = Self(args);
            let db_config = match (cmd_args.hybrid(), cmd_args.document()) {
                (true, false) => DbConfig::hybrid(),
                (false, true) => DbConfig::document(cmd_args.compress()),
                _ => DbConfig::normalize(),
            };
            // this is safe, the thread parse already confirms it's a usize
            let threads = cmd_args.threads();
            let drop = cmd_args.drop();
            let load = cmd_args.load();
            admin_api.init(db_config, drop, load, threads)?;
            Ok(())
        }
        /// Used by the command parser to validate the thread count argument.
        ///
        /// Yeah, I know you can use a built in but the error message was bugging me.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the weather directory command argument.
        fn thread_count_parse(count_arg: &str) -> result::Result<usize, String> {
            match count_arg.parse::<usize>() {
                Ok(count) => {
                    let max_threads = 16;
                    if count <= max_threads {
                        Ok(count)
                    } else {
                        Err(format!("thread count is limited to {}.", max_threads))
                    }
                }
                Err(_) => Err(format!("{} is not a number.", count_arg)),
            }
        }
        /// Get the hybrid command flag.
        fn hybrid(&self) -> bool {
            self.0.get_flag(Self::HYBRID)
        }
        /// Get the document command flag.
        fn document(&self) -> bool {
            self.0.get_flag(Self::DOCUMENT)
        }
        /// Get the compress command flag.
        fn compress(&self) -> bool {
            self.0.get_flag(Self::COMPRESS)
        }
        /// Get the threads command flag.
        fn threads(&self) -> usize {
            *self.0.get_one(Self::THREADS).unwrap()
        }
        /// Get the drop command flag.
        fn drop(&self) -> bool {
            self.0.get_flag(Self::DROP)
        }
        /// Get the load command flag.
        fn load(&self) -> bool {
            self.0.get_flag(Self::LOAD)
        }
    }

    /// The drop command
    #[derive(Debug)]
    struct DropCmd(
        /// The drop command arguments
        ArgMatches,
    );
    impl DropCmd {
        /// The drop sub-command name.
        const NAME: &str = "drop";
        /// The command argument id to remove the existing weather data database file.
        const DELETE: &str = "DELETE";
        /// Get the drop sub-command definition.
        fn get() -> Command {
            Command::new(Self::NAME).about("Delete the existing database schema.").arg(
                Arg::new(Self::DELETE)
                    .long("delete")
                    .action(ArgAction::SetTrue)
                    .help("Remove the database file from the weather data directory."),
            )
        }
        /// Collect the command line arguments and run the drop database sub-command.
        ///
        /// # Arguments
        ///
        /// * `admin_api` is the backend weather adminstration `API`.
        /// * `args` holds the drop command arguments.
        fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
            let cmd_args = Self(args);
            let delete = cmd_args.delete();
            Ok(admin_api.drop(delete)?)
        }
        /// Get the delete command flag.
        fn delete(&self) -> bool {
            self.0.get_flag(Self::DELETE)
        }
    }

    /// The migrate command.
    #[derive(Debug)]
    struct MigrateCmd(
        /// The migrate command arguments
        ArgMatches,
    );
    impl MigrateCmd {
        /// The drop sub-command name.
        const NAME: &str = "migrate";
        /// The command argument id for the target directory.
        const INTO_DIR: &str = "INTO";
        /// The command argument id for the create directory argument.
        const CREATE: &str = "CREATE";
        /// The command argument id for retaining data in converted arhives.
        const RETAIN: &str = "RETAIN";
        /// The command argument id for which archives should be migrated.
        const CRITERIA: &str = "CRITERIA";
        /// Get the migrate sub-command definition.
        fn get() -> Command {
            Command::new(Self::NAME)
                .about("Migrate DarkSky archives to internal weather history.")
                .arg(
                    Arg::new(Self::INTO_DIR)
                        .long("into")
                        .required(true)
                        .action(ArgAction::Set)
                        .value_parser(Self::into_dir_parser)
                        .help("The directory where converted archives will be written."),
                )
                .arg(
                    Arg::new(Self::CREATE)
                        .short('c')
                        .long("create")
                        .action(ArgAction::SetTrue)
                        .help(format!("Create <{}> if it does not exist.", Self::INTO_DIR)),
                )
                .arg(
                    Arg::new(Self::RETAIN)
                        .short('r')
                        .long("retain")
                        .action(ArgAction::SetTrue)
                        .help(format!("Do not overwrite existing archives in <{}>.", Self::INTO_DIR)),
                )
                .arg(
                    Arg::new(Self::CRITERIA)
                        .action(ArgAction::Append)
                        .help("Limit migration to select archives (supports wildcards)."),
                )
        }
        /// Collect the command line arguments and run the migrate command.
        ///
        /// # Arguments
        ///
        /// * `admin_api` is the backend weather adminstration `API`.
        /// * `args` is the migrate command arguments.
        fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
            let cmd_args = Self(args);
            let into = cmd_args.into_dir();
            let create = cmd_args.create();
            let retain = cmd_args.retain();
            let criteria = DataCriteria { filters: cmd_args.criteria(), icase: true, sort: false };
            let convert_count = admin_api.migrate(into, create, retain, criteria)?;
            log::info!("{} archives converted.", convert_count);
            Ok(())
        }
        /// Validate the directory archives will be migrated into.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the into directory command line argument.
        fn into_dir_parser(dirname: &str) -> result::Result<PathBuf, String> {
            if dirname.is_empty() {
                Err("The directory name cannot be empty.".to_string())
            } else {
                let filepath = PathBuf::from(dirname);
                if filepath.is_dir() {
                    Ok(filepath)
                } else if filepath.exists() {
                    Err(format!("{} must be a directory name.", dirname))
                } else {
                    Ok(filepath)
                }
            }
        }
        /// Get the retain command flag.
        fn retain(&self) -> bool {
            self.0.get_flag(Self::RETAIN)
        }
        /// Get the create command flag.
        fn create(&self) -> bool {
            self.0.get_flag(Self::CREATE)
        }
        /// Get the into command argument.
        fn into_dir(&self) -> PathBuf {
            self.0.get_one::<PathBuf>(Self::INTO_DIR).unwrap().clone()
        }
        /// Get the hybrid command arguments.
        fn criteria(&self) -> Vec<String> {
            match self.0.get_many::<String>(Self::CRITERIA) {
                Some(filters) => filters.map(|f| f.clone()).collect(),
                None => vec![],
            }
        }
    }

    /// The stat command.
    struct StatCmd;
    impl StatCmd {
        /// The stat sub-command name.
        const NAME: &str = "stat";
        /// Get the drop sub-command definition.
        fn get() -> Command {
            Command::new(Self::NAME).about("Get metrics about the weather data database.")
        }
        /// Collect the command line arguments and run the stat database sub-command.
        ///
        /// # Arguments
        ///
        /// * `admin_api` is the backend weather adminstation `API`.
        fn run(admin_api: &WeatherAdmin, _: ArgMatches) -> Result<()> {
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
        fn get() -> Vec<Arg> {
            vec![
                Arg::new(Self::WEATHER_DIR)
                    .short('d')
                    .long("directory")
                    .action(ArgAction::Set)
                    .value_parser(Self::parse_weather_dir)
                    .value_name("DIR")
                    .help("The weather data directory pathname."),
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
        fn weather_dir(&self) -> &str {
            self.0.get_one::<String>(Self::WEATHER_DIR).map_or(Default::default(), |s| s.as_str())
        }
        /// Get the logfile name argument.
        fn logfile(&self) -> Option<PathBuf> {
            self.0.get_one::<PathBuf>(Self::LOGFILE).map_or(Default::default(), |p| Some(p.clone()))
        }
        /// Get the append to logfile flag.
        fn append(&self) -> bool {
            self.0.get_flag(Self::APPEND)
        }
        /// Get the logging verbosity flag.
        fn verbosity(&self) -> u8 {
            std::cmp::min(self.0.get_one::<u8>(Self::VERBOSITY).map_or(0, |a| *a), 3)
        }
    }
    impl<'a> From<&'a ArgMatches> for CommandArgs<'a> {
        fn from(args: &'a ArgMatches) -> Self {
            Self(args)
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

    #[cfg(test)]
    mod tests {
        use super::*;

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
            assert_eq!(command_args.verbosity(), 0);
            let known_dir = env!("CARGO_MANIFEST_DIR");
            let matches = arg_matches(&mut cmd, &["testcase", "-d", known_dir, "-l", "logfile", "-a", "-vvvv"]);
            let command_args = CommandArgs(&matches);
            assert_eq!(command_args.weather_dir(), known_dir);
            assert_eq!(command_args.logfile().unwrap(), PathBuf::from("logfile"));
            assert!(command_args.append());
            assert_eq!(command_args.verbosity(), 3)
        }
    }
}
