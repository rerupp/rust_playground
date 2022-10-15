//! # Add a filesystem folder hierarchy to a database.
use std::path::PathBuf;

use clap::Args;

use super::{result, Session, Result, StopWatch};

/// The load database command arguments.
#[derive(Args, Debug)]
pub struct CommandArgs {
    /// A filesystem directory that will be traversed and loaded into the database.
    #[clap(forbid_empty_values = true, parse(try_from_str = parse_dir_name))]
    folder_path: PathBuf,
}

/// Used by the `clap` API to convert the CLI argument into a `PathBuf`.
fn parse_dir_name(dir_name: &str) -> result::Result<PathBuf, String> {
    if dir_name.trim().len() != dir_name.len() {
        Err("The directory cannot have leading/trailing white space...".to_string())
    } else {
        let dirpath = PathBuf::from(dir_name);
        if dirpath.is_dir() {
            Ok(dirpath)
        } else if dirpath.exists() {
            Err(format!("{} must be a directory name...", dir_name))
        } else {
            Err(format!("The directory does not exist..."))
        }
    }
}

/// The load database command definition.
pub struct Command {
    /// The command arguments.
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

    /// Uses a [Session] from `fsviewlib` to call the API that will add folder metadata to the database.
    /// 
    /// # Arguments
    /// 
    /// * `session` - the `domain` session that will be used to add the metadata.
    pub fn execute(&self, session: &Session) -> Result<()> {
        let elapsed = StopWatch::start_new();
        session.add_folder(&self.args.folder_path)?;
        println!("overall={elapsed}");
        Ok(())
    }
}
