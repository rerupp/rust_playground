use clap::Parser;
mod cli;

/// The `fsview` executable entry point.
/// 
/// Its role is to parse the command line arguments and execute the appropriate CLI command.
fn main() -> Result<(), cli::Error> {
    let cmd: cli::Cli = cli::Cli::parse();
    cli::initialize(&cmd)?;
    cli::execute(cmd)
}
