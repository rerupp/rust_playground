//! The sync database with archives command.
use super::*;
// use clap::{Arg, ArgAction, ArgMatches, Command};
// use weather_lib::admin_prelude::WeatherAdmin;
use weather_lib::prelude::DataCriteria;

pub(super) use v3::ReloadCmd;
mod v3 {
    //! The current version of the reload command.
    use super::*;

    #[derive(Debug)]
    pub struct ReloadCmd(
        /// The sync command arguments
        ArgMatches,
    );

    impl ReloadCmd {
        /// The sync sub-command name.
        pub const NAME: &'static str = "reload";
        /// The command argument id for which archives should be synced.
        const CRITERIA: &'static str = "CRITERIA";
        /// Get the migrate sub-command definition.
        pub fn get() -> Command {
            Command::new(Self::NAME).about("Reload database weather history for locations.").arg(
                Arg::new(Self::CRITERIA)
                    .value_name("LOCATION")
                    .action(ArgAction::Append)
                    .required(true)
                    .help("The locations that will be reloaded (supports wildcards)."),
            )
        }
        /// Collect the command line arguments and run the migrate command.
        ///
        /// # Arguments
        ///
        /// * `admin_api` is the backend weather administration `API`.
        /// * `args` is the migrate command arguments.
        pub fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
            let cmd_args = Self(args);
            let criteria = DataCriteria { filters: cmd_args.criteria(), icase: true, sort: false };
            let sync_count = admin_api.reload(criteria)?;
            log::info!("{} archives converted.", sync_count);
            Ok(())
        }
        /// Get the hybrid command arguments.
        fn criteria(&self) -> Vec<String> {
            match self.0.get_many::<String>(Self::CRITERIA) {
                Some(filters) => filters.map(|f| f.clone()).collect(),
                None => vec![],
            }
        }
    }
}