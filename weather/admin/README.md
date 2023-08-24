# Weather Data Administration CLI

### The `admin` Program

This is the database initialization utility for weather data. It uses the `WeatherAdmin` API defined in `weather_lib` to execute commands. The `main` function parses the command line, initializes logging, and executes the appropriate sub-command.

The default logging level is `WARN`. `INFO`, `DEBUG`, and `TRACE` logging levels will be set depending on the command verbosity.

### The `cli` Module

The CLI is built on top of the `clap` crate. The `Command` API is used to implement the main command arguments and sub-commands. This is different from the `weather` CLI that uses derive attributes for structures. Staying away from structures allowed the implementation to be function based. 

I also found modifying command and sub-command arguments to be much easier than using derive attributes. When looking at the implementation in code, it seems much cleaner and more consise as well.

Here's an overview of the adminstration commands.

```
$ admin
The weather data administration tool.

Usage: admin [OPTIONS] <COMMAND>

Commands:
  init  Initialize the weather data database.
  drop  Removes the existing database schema.
  stat  Get metrics about the weather data database.
  help  Print this message or the help of the given subcommand(s)

Options:
  -d, --directory <DIR>  The weather data directory pathname.
  -l, --logfile <LOG>    The log filename (DEFAULT stdout).
  -a, --append           Append to the logfile, otherwise overwrite.
  -v, --verbose...       Logging verbosity (once=INFO, twice=DEBUG, +twice=TRACE)
  -h, --help             Print help
  -V, --version          Print version
  ```

Help for subcommands is also available.

```
$ admin init -h
Initialize the weather data database.

Usage: admin init [OPTIONS]

Options:
      --hybrid    Configure the database to use archives for history data (default).
      --document  Configure the database to use JSON for history data.
      --compress  The JSON history data will be compressed in the database.
      --full      Configure the database to be fully relational.
      --drop      Drops the database before initializing.
      --load      Load the database after initializing.
  -h, --help      Print help
  ```
  
