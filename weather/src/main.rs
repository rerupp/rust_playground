use clap::Parser;

use weather::{cli, data};
use weather::domain::WeatherData;

fn main() -> cli::CliResult<()> {
    let cmd: cli::Cli = cli::Cli::parse();
    let data_api = data::from_pathname(cmd.data_dir())?;
    let weather_data = WeatherData::new(data_api);
    cli::dispatch(cmd, &weather_data)
}
