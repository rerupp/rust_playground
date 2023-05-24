mod cli;
use clap::Parser;
use std::path::PathBuf;
use weather_lib as lib;
use toolslib::logs::{initialize as log_initialize, LogProperties};

fn main() -> cli::Result<()> {
    let cmd: cli::Cli = cli::Cli::parse();
    // let data_api = data::from_pathname(cmd.data_dir())?;
    // let weather_data = domain::WeatherData::new(data_api);
    initialize(&cmd);
    let files_path = PathBuf::from(cmd.data_dir());
    let weather_data = lib::weather_data(&files_path)?;
    cli::dispatch(cmd, &weather_data)
}

/// Prepares the CLI for execution of commands. This really needs to go somewhere else but
/// right now it's better than having it internal to execute.
// pub fn initialize(_cmd: &cli::Cli) -> lib::Result<()> {
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
        file_loggers: vec![String::from("toolslib"), String::from("weather")],
    }) {
        Ok(_) => (),
        Err(log_error) => eprintln!("Error initializing logging!!! {:?}", log_error),
    };
}
