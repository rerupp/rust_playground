//! # The `fsview` command line interface (CLI).
//! 
//! The `cli` module provides a front end CLI to interact with the filesystem.
//! It uses the [fsviewlib] library to load and query information about folders
//! and files.
//! 
//! The CLI is built with `clap` and uses *`derive`* code mark up to define the
//! commands and their arguments. It also makes use of modules in the `toolslib`
//! library to provide timing information and initialize the logging subsystem.
use clap::{AppSettings, ArgAction, Parser, Subcommand};
use fsviewlib as lib;
use lib::{DbInformation, Metadata, Session};
use log4rs::Handle;
use std::{
    fmt,
    path::PathBuf,
    result,
};
use toolslib::{
    logs::{initialize as log_initialize, LogError, LogProperties},
    stopwatch::StopWatch,
};

mod init_database;
mod list_folders;
mod load_database;

// The result of calling a CLI function.
type Result<T> = result::Result<T, Error>;

/// An error specific to the CLI module.
#[derive(Debug)]
pub struct Error(String);

/// Satisfy that errors need to implement the `Display` trait.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Convert a `fsview` library error to a CLI error.
impl From<lib::Error> for Error {
    fn from(error: lib::Error) -> Self {
        Error(format!("fsview: {error}"))
    }
}

/// Convert a `toolslib` library error to a CLI error.
impl From<LogError> for Error {
    fn from(error: LogError) -> Self {
        Error(format!("toolslib: {error}"))
    }
}

/// Create a CLI error from the string slice.
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error::from(String::from(error))
    }
}

/// Create a CLI error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(format!("cli: {error}"))
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
        forbid_empty_values = true, parse(try_from_str = parse_logfile_name),
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
fn parse_logfile_name(log_filename: &str) -> result::Result<PathBuf, String> {
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
        #[clap(flatten)]
        args: init_database::CommandArgs,
    },

    /// Lists folder content and metadata.
    #[clap(name="list", setting=AppSettings::DeriveDisplayOrder)]
    QueryFolder {
        #[clap(flatten)]
        args: list_folders::CommandArgs,
    },

    /// Loads database with folder metadata.
    #[clap(name="load", setting=AppSettings::DeriveDisplayOrder)]
    LoadDatabase {
        #[clap(flatten)]
        args: load_database::CommandArgs,
    },
}

/// Prepares the CLI for execution of commands. This really needs to go somewhere else but
/// right now it's better than having it internal to execute.
pub fn initialize(cmd: &Cli) -> Result<Handle> {
    let handle = log_initialize(LogProperties {
        level: match cmd.verbosity {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        },
        console_pattern: None,
        logfile_pattern: None,
        logfile_path: cmd.logfile_path.clone(),
        logfile_append: cmd.append_log,
        file_loggers: vec![String::from("toolslib"), String::from("fsview")],
    })?;
    Ok(handle)
}

/// Executes the CLI command.
pub fn execute(cmd: Cli) -> Result<()> {
    let session = lib::get_session(cmd.db_path)?;
    match cmd.command {
        Some(Commands::InitDatabase { args }) => {
            let init_database = init_database::Command::new(args);
            init_database.execute(&session)
        }
        Some(Commands::LoadDatabase { args }) => {
            let load_database = load_database::Command::new(args);
            load_database.execute(&session)
        }

        Some(Commands::QueryFolder { args }) => {
            let list_folders = list_folders::Command::new(args);
            list_folders.execute(&session)
        }

        _ => Err(Error::from("Command not recognized!!!")),
    }
}
