mod cli;
use clap::Parser;
use toolslib::logs::{initialize as log_initialize, LogProperties};
use weather_lib::{archive_weather_data, db_weather_data};

/// The weather cli entry point.
fn main() -> Result<(), String> {
    // move to try_parse()
    let cmd: cli::Cli = cli::Cli::parse();
    match run(cmd) {
        Ok(_) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Initialize and execute the parsed command line.
fn run(cmd: cli::Cli) -> cli::Result<()> {
    initialize(&cmd);
    let weather_data = if cmd.db {
        db_weather_data(&cmd.data_dir())?
    } else {
        archive_weather_data(&cmd.data_dir())?
    };
    cli::dispatch(cmd, &weather_data)
}

/// Prepares the CLI for execution of commands. This really needs to go somewhere else but
/// right now it's better than having it internal to execute.
pub fn initialize(cmd: &cli::Cli) {
    match log_initialize(LogProperties {
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
        file_loggers: vec![String::from("toolslib"), String::from("weather"), String::from("weather_lib")],
    }) {
        Ok(_) => (),
        Err(log_error) => eprintln!("Error initializing logging!!! {:?}", log_error),
    };
}
