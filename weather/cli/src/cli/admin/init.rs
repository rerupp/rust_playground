//! The weather data initialization command.
use super::*;

pub(super) use v3::InitCmd;
mod v3 {
    //! The latest version of weather data initialization.
    use super::*;
    use weather_lib::admin_prelude::DbMode;

    #[derive(Debug)]
    pub struct InitCmd(
        /// The init command arguments.
        ArgMatches,
    );
    impl InitCmd {
        /// The initialize sub-command name.
        pub const NAME: &'static str = "init";
        /// The command argument id for running in hybrid mode.
        const HYBRID: &'static str = "HYBRID";
        /// The command argument id for running in `JSON` document mode.
        const DOCUMENT: &'static str = "DOCUMENT";
        /// The command argument id for running in a full normalized mode.
        const NORMALIZE: &'static str = "NORMALIZE";
        /// The command argument id indicating the database schema should be dropped.
        const DROP: &'static str = "DROP";
        /// The command argument id indicating the database should be loaded.
        const LOAD: &'static str = "LOAD";
        /// The command argument id indicating `JSON` documents should be compressed in the database.
        const COMPRESS: &'static str = "COMPRESS";
        /// The command argument id controlling how many threads to use.
        const THREADS: &'static str = "THREADS";
        /// Get the initialize sub-command definition.
        pub fn get() -> Command {
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
                        .long("normalized")
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
        pub fn run(admin_api: &WeatherAdmin, args: ArgMatches) -> Result<()> {
            let cmd_args = Self(args);
            let db_mode = match (cmd_args.hybrid(), cmd_args.document()) {
                (true, false) => DbMode::Hybrid,
                (false, true) => DbMode::Document(cmd_args.compress()),
                _ => DbMode::Normalized,
            };
            // this is safe, the thread parse already confirms it's a usize
            let threads = cmd_args.threads();
            let drop = cmd_args.drop();
            let load = cmd_args.load();
            admin_api.init(db_mode, drop, load, threads)?;
            Ok(())
        }
        /// Used by the command parser to validate the thread count argument.
        ///
        /// Yeah, I know you can use a built in but the error message was bugging me.
        ///
        /// # Arguments
        ///
        /// * `dirname` is the weather directory command argument.
        fn thread_count_parse(count_arg: &str) -> std::result::Result<usize, String> {
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
}
