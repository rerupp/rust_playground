//! The add weather data history command.

use super::*;

/// The add weather data history command name.
pub const COMMAND_NAME: &'static str = "ah";

pub use v4::{command, execute};
mod v4 {
    //! The current version of add weather data history.
    use super::*;
    use chrono::NaiveDate;

    /// The location argument id.
    const LOCATION: &'static str = "LOCATION";

    /// The history from date argument id.
    const FROM: &'static str = "FROM";

    /// The history thru date argument id.
    const THRU: &'static str = "THRU";

    /// Create a new instance of the add history command arguments.
    ///
    pub fn command() -> Command {
        Command::new(COMMAND_NAME)
            .about("Add weather history to a location.")
            .arg(
                Arg::new(LOCATION)
                    .action(ArgAction::Set)
                    .required(true)
                    .value_name("LOCATION")
                    .value_parser(validate_location)
                    .help("The location weather history will be added to."),
            )
            .arg(
                Arg::new(FROM)
                    .action(ArgAction::Set)
                    .required(true)
                    .value_parser(date_parser)
                    .value_name("FROM")
                    .help("The weather history starting date."),
            )
            .arg(
                Arg::new(THRU)
                    .action(ArgAction::Set)
                    .required(false)
                    .value_parser(date_parser)
                    .value_name("THRU")
                    .help("The weather history ending date."),
            )
            .arg_required_else_help(true)
    }

    /// Executes the add history command.
    ///
    /// # Arguments
    ///
    /// * `weather_data` is the weather library API used by the command.
    /// * `args` contains the report history command arguments.
    ///
    pub fn execute(weather_data: &WeatherData, args: ArgMatches) -> Result<()> {
        macro_rules! error {
            ($reason:expr) => {
                Err(Error::from($reason.to_string()))
            };
        }
        let location = args.get_one::<String>(LOCATION).unwrap().clone();
        match weather_data.get_locations(DataCriteria { filters: vec![location.clone()], icase: true, sort: false }) {
            Err(error) => error!(format!("Error getting location '{}' information ({}).", location, error)),
            Ok(mut locations) => {
                let len = locations.len();
                if len == 0 {
                    error!(format!("Location '{}' was not found.", location))
                } else if len > 1 {
                    error!(format!("Multiple locations were found for '{}'.", location))
                } else {
                    let location = locations.pop().unwrap();
                    let from = args.get_one::<NaiveDate>(FROM).unwrap();
                    let to = args.get_one::<NaiveDate>(THRU).map_or(from, |d| d);
                    let date_range = DateRange { from: from.clone(), to: to.clone() };
                    match weather_data.get_history_client() {
                        Err(error) => error!(error.to_string()),
                        Ok(client) => {
                            let daily_histories = get_histories(&client, location, date_range)?;
                            let histories_found = daily_histories.histories.len();
                            let histories_added = weather_data.add_histories(daily_histories)?;
                            println!("\n{} histories found, {} histories added.", histories_found, histories_added);
                            Ok(())
                        }
                    }
                }
            }
        }
    }
}

/// This function manages calling the history client and providing a hint on the request progress.
///
/// # Arguments
///
/// - `client` is the history client.
/// - `location` is the historical weather data owner.
/// - `date_range` are the dates being asked for.
///
fn get_histories(client: &Box<dyn HistoryClient>, location: Location, date_range: DateRange) -> Result<DailyHistories> {
    use std::io::stdout;
    use std::thread::sleep;
    use std::time::{Duration, SystemTime};
    client.execute(&location, &date_range)?;
    let timeout = SystemTime::now() + Duration::new(30, 0);
    let pause = Duration::from_millis(10);
    let mut loop_cnt = 0usize;
    // this loop could use some tender love
    loop {
        if SystemTime::now() > timeout {
            Err(Error::from("Client history timed out"))?;
        }
        if (loop_cnt % 20) == 0 {
            write!(stdout().lock(), ".").unwrap();
            stdout().flush().unwrap();
        }
        loop_cnt += 1;
        if client.poll()? {
            break;
        }
        sleep(pause);
    }
    // poll() breaks the loop so this will not hang the commandline
    match client.get() {
        Ok(daily_histories) => Ok(daily_histories),
        Err(error) => Err(Error::from(error)),
    }
}
