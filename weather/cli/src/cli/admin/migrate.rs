//! The weather data administration migrate command.
use crate::cli::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;
use weather_lib::{
    admin_prelude::WeatherAdmin,
    prelude::DataCriteria
};

pub(in crate::cli::admin) use v3::MigrateCmd;
mod v3 {
    //! The current version of the migrate command.
    use super::*;

    #[derive(Debug)]
    pub struct MigrateCmd(
        /// The migrate command arguments
        ArgMatches,
    );

    impl MigrateCmd {
        /// The migrate sub-command name.
        pub const NAME: &'static str = "migrate";
        /// The command argument id for the target directory.
        const INTO_DIR: &'static str = "INTO";
        /// The command argument id for the create directory argument.
        const CREATE: &'static str = "CREATE";
        /// The command argument id for retaining data in converted archives.
        const RETAIN: &'static str = "RETAIN";
        /// The command argument id for which archives should be migrated.
        const CRITERIA: &'static str = "CRITERIA";
        /// Get the migrate sub-command definition.
        pub fn get() -> Command {
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
        /// * `admin_api` is the backend weather administration `API`.
        /// * `args` is the migrate command arguments.
        pub fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
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
        fn into_dir_parser(dirname: &str) -> std::result::Result<PathBuf, String> {
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
}