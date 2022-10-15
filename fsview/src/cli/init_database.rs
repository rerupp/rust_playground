//! # The subcommand that initializes a database.
//!
use clap::Args;

use super::{Session, Result, StopWatch};

/// The initialize database command arguments.
#[derive(Args, Debug)]
pub struct CommandArgs {
    /// Inidcates if the database should be dropped before updating the schema.
    #[clap(short, long)]
    drop: bool,
}

/// The initialize database command definition.
pub struct Command {
    /// The commands arguments.
    args: CommandArgs,
}

impl Command {

    /// Creates an instance of the command.
    /// 
    /// # Arguments
    /// 
    /// * `args` - the command arguments that will be used.
    pub fn new(args: CommandArgs) -> Command {
        Command { args }
    }

    /// Uses a [Session] from `fsviewlib` to call the API that will initialize a database.
    /// 
    /// # Arguments
    /// 
    /// * `session` - the `domain` session that will be called to initialize the database.
    pub fn execute(&self, session: &Session) -> Result<()> {
        let elapsed = StopWatch::start_new();
        session.initialize_db(self.args.drop)?;
        log::info!("initialize took {elapsed}");
        Ok(())
    }
}
