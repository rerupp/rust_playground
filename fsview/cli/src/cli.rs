//! # The `fsview` command line interface (CLI).
//!
//! The `cli` module provides a front end CLI to interact with the filesystem.
//! It uses the [fs] library to load and query information about folders
//! and files.
//!
//! The CLI is built with `clap` and uses *`derive`* code mark up to define the
//! commands and their arguments. It also makes use of modules in the `toolslib`
//! library to provide timing information and initialize the logging subsystem.
use clap::{AppSettings, ArgAction, Parser, Subcommand};
use fs as lib;
use lib::domain::{get_session, FolderMd, Metadata, Session};
use log4rs::Handle;
use std::{fmt, io, path::PathBuf, result};
use toolslib::{
    fmt::commafy,
    logs::{initialize as log_initialize, LogError, LogProperties},
    mbufmt, rptcols, rptrow,
    stopwatch::StopWatch,
    text,
};

mod file_duplicates;
mod init_database;
mod list_folders;
mod load_database;

/// The result of calling a CLI function.
type Result<T> = result::Result<T, Error>;

/// An error specific to the CLI module.
#[derive(Debug)]
pub struct Error(String);
/// Include the [`ToString`] trait for the domain [`Error`].
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// The error conversion all error handlers should wind up calling.
impl From<&str> for Error {
    fn from(error_text: &str) -> Self {
        Error(format!("fsview {error_text}"))
    }
}
/// Create a CLI error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(format!("CLI: {error}").as_str())
    }
}
/// Convert a `fsview` library error to a CLI error.
impl From<lib::domain::Error> for Error {
    fn from(error: lib::domain::Error) -> Self {
        Error::from(format!("library: {error}").as_str())
    }
}
/// Convert a `toolslib` library error to a CLI error.
impl From<LogError> for Error {
    fn from(error: LogError) -> Self {
        Error::from(format!("toolslib: {error}").as_str())
    }
}
/// Create a CLI error from a string.
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::from(format!("IO: {error}").as_str())
    }
}
/// Create a CLI error from a `fmt::Error`.
impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error::from(format!("fmt: {error}").as_str())
    }
}
/// Create a CLI error from a toolslib `text::Error`.
impl From<text::Error> for Error {
    fn from(error: text::Error) -> Self {
        Error::from(format!("toolslib{error}").as_str())
    }
}

/// The command line interface (CLI) properties and commands.
#[derive(Parser, Debug)]
#[clap(bin_name = "fsview", setting = AppSettings::ArgRequiredElseHelp)]
pub struct Cli {
    /// The name of the database that will be used.
    #[clap(long = "db", value_name="DB", forbid_empty_values = true, parse(try_from_str = parse_db_name), display_order = 1)]
    pub db_path: Option<PathBuf>,
    /// The filename logging output will be written into
    #[clap(
        long = "log", value_name="LOG",
        forbid_empty_values = true, parse(try_from_str = parse_filename),
        group="log", display_order = 2
    )]
    /// The log file pathname.
    pub logfile_path: Option<PathBuf>,
    /// Append to the log file, otherwise overwrite
    #[clap(short, long = "append", requires("log"), display_order = 3)]
    pub append_log: bool,
    /// Logging verbosity level (once=INFO, twice=DEBUG, thrice=TRACE)
    #[clap(short, long, action(ArgAction::Count), display_order = 4)]
    verbosity: u8,
    /// The command supported by the CLI.
    #[clap[subcommand]]
    pub command: Option<Commands>,
}

/// Used by `clap` to validate the log file path.
///
/// # Arguments
///
/// * `log_filename` is the path to a file that will be used for logging output.
fn parse_filename(log_filename: &str) -> result::Result<PathBuf, String> {
    if log_filename.trim().len() != log_filename.len() {
        Err("The log filename cannot have leading/trailing white space...".to_string())
    } else {
        let log_path = PathBuf::from(log_filename);
        if log_path.is_dir() {
            Err("The log filename cannot be a directory...".to_string())
        } else {
            Ok(log_path)
        }
    }
}

/// Used by `clap` to validate the database filename.
///
/// # Arguments
///
/// * `db_name` is the path to the database that will be used to access file metadata.
fn parse_db_name(db_name: &str) -> result::Result<PathBuf, String> {
    if db_name.trim().len() != db_name.len() {
        Err("The DB name cannot have leading/trailing white space...".to_string())
    } else {
        let db_path = PathBuf::from(db_name);
        if db_path.is_dir() {
            Err("The DB name cannot be a directory...".to_string())
        } else {
            Ok(db_path)
        }
    }
}

/// The commands supported by the CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize the database schema.
    #[clap(name="init", setting=AppSettings::DeriveDisplayOrder)]
    InitDatabase {
        /// The [`command arguments`](init_database::CommandArgs) used to initialize the database.
        #[clap(flatten)]
        args: init_database::CommandArgs,
    },
    /// Reports folder metadata and database information.
    #[clap(name="list", setting=AppSettings::DeriveDisplayOrder)]
    ListFolder {
        /// The [`command arguments`](list_folders::CommandArgs) used to show folder metadata.
        #[clap(flatten)]
        args: list_folders::CommandArgs,
    },
    /// Loads database with folder metadata.
    #[clap(name="load", setting=AppSettings::DeriveDisplayOrder)]
    LoadDatabase {
        /// The [`command arguments`](load_database::CommandArgs) used to load folder metadata.
        #[clap(flatten)]
        args: load_database::CommandArgs,
    },
    /// Initializes or reports on duplicate files
    #[clap(name="dups", setting=AppSettings::DeriveDisplayOrder)]
    FileDuplicates {
        /// The [`command arguments`](file_duplicates::CommandArgs) used to show duplicate file metadata.
        #[clap(flatten)]
        args: file_duplicates::CommandArgs,
    },
}

/// Prepares the CLI for execution of commands. This really needs to go somewhere else but
/// right now it's better than having it internal to execute.
///
/// # Arguments
///
/// * `cli` arguments will be used to initialize the CLI.
pub fn initialize(cli: &Cli) -> Result<Handle> {
    let handle = log_initialize(LogProperties {
        level: match cli.verbosity {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        },
        console_pattern: None,
        logfile_pattern: None,
        logfile_path: cli.logfile_path.clone(),
        logfile_append: cli.append_log,
        file_loggers: vec![String::from("toolslib"), String::from("fsview")],
    })?;
    Ok(handle)
}

/// Executes the CLI command.
///
/// # Arguments
///
/// * `cli` contains the subcommand that will be run.
pub fn execute(cli: Cli) -> Result<()> {
    let db_path = match cli.db_path {
        Some(db_name) => db_name,
        None => {
            let package_name = option_env!("CARGO_PKG_NAME");
            PathBuf::from(package_name.unwrap_or("fsview")).with_extension("db")
        },
    };
    let session = get_session(db_path)?;
    match cli.command {
        Some(Commands::InitDatabase { args }) => {
            let init_database = init_database::Command::new(args);
            init_database.execute(&session)
        }
        Some(Commands::LoadDatabase { args }) => {
            let load_database = load_database::Command::new(args);
            load_database.execute(&session)
        }
        Some(Commands::ListFolder { args }) => {
            let list_folders = list_folders::Command::new(args);
            list_folders.execute(&session)
        }
        Some(Commands::FileDuplicates { args }) => {
            let file_duplicates = file_duplicates::Command::new(args);
            file_duplicates.execute(&session)
        }
        _ => Err(Error::from("Command not recognized!!!")),
    }
}
