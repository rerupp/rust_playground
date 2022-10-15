mod cli;
use clap::Parser;
use std::path::PathBuf;
use weather as lib;

fn main() -> cli::Result<()> {
    let cmd: cli::Cli = cli::Cli::parse();
    // let data_api = data::from_pathname(cmd.data_dir())?;
    // let weather_data = domain::WeatherData::new(data_api);
    let files_path = PathBuf::from(cmd.data_dir());
    let weather_data = lib::weather_data(&files_path)?;
    cli::dispatch(cmd, &weather_data)
}
