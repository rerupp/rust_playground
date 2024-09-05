# Weather Data CLI

### The `main` program

The `main` program is responsible for bootstrapping the CLI and passing control to it.
The `main` program  has been changed to return `ExitStatus` since it best describes the
termination of a program.

### The `cli` Module

The CLI is built on top of the `clap` crate using their program API. Originally I used `#derive` 
class attributes however it quickly got bloated and complex (read this as a lot of crap). I 
really like the program centric implementation patterns `clap` encourages and to me feels much more 
consise.

Some differences from the last release is that `admin` commands have been moved into the CLI and 
is no longer a standalone program. After moving to the `clap` program API, encorporating the  
administration API into the main program made this pretty easy.


The bootstrap of the CLI continues to include setting up a logger. The verbosity of the logger  
(`INFO`, `DEBUG`, or `TRACE`) can be changed via a command line argument. The default level is  
`WARN`.

Most of the commands support producing report output in the form of plain text, `JSON`, or `CSV` 
formats. Command line arguments allow the reports to be saved to a file instead of being output  
to `stdout`.

#### The `cli::admin` module

The `admin` module contains the administration CLI commands.

#### The `cli::reports` module

The `reports` module contains the various reports avaliable to the CLI commands. Previously this
was buried in the command implementation but was moved due to the TUI addition.

#### The `cli::tui` module

The `tui` module contains the terminal based UI application. This was one of the big additions in 
for the release. All of the non-admin commands have been incorporated into the TUI. The TUI is 
built on top of [terminal UI](../termui/README.md) library.

The TUI main window consists of a menu bar with tabbed windows showing the locations, summary, or 
history reports. Only textual report output is available as of now. A weather data location can be 
added along with adding historical weather data.

#### The `cli::user` module

The `user` module contains the non-administration CLI commands.

### The `weather` application.

The weather executable consists of various subcommands. If a subcommand is not entered, a help 
overview is provided.

```
$ weather
The weather data command line.

Usage: weather [OPTIONS] <COMMAND>

Commands:
  ll     List the known weather data history locations_win.
  lh     List the dates of weather history available by location.
  ls     List a summary of weather data available by location.
  rh     Generate a weather history report for a location.
  ah     Add weather history to a location.
  tui    A Terminal based weather data UI.
  admin  The weather data administration tool.
  help   Print this message or the help of the given subcommand(s)

Options:
  -c, --config <FILE>    The configuration file pathname (DEFAULT weather.toml).
  -d, --directory <DIR>  The weather data directory pathname.
      --fs               Do not use a weather history DB if one is available.
  -l, --logfile <FILE>   The log filename (DEFAULT stdout).
  -a, --append           Append to the logfile, otherwise overwrite.
  -v, --verbose...       Logging verbosity (once=INFO, twice=DEBUG, +twice=TRACE)
  -h, --help             Print help
  -V, --version          Print version
```

Help for subcommands are also available.

```
$ weather rh
Generate a weather history report for a location.

Usage: weather rh [OPTIONS] <LOCATION> <FROM> [THRU]

Arguments:
  <LOCATION>  The location to use for the weather history.
  <FROM>      The weather history starting date.
  [THRU]      The weather history ending date.

Options:
  -t, --temp           Include temperature information in the report (default).
  -p, --precip         Include percipitation information in the report.
  -c, --cnd            Include weather conditions in the report.
  -s, --sum            Include summary information in the report.
  -a, --all            Include all weather information in the report.
      --text           The report will be plain Text (default)
      --csv            The report will be in CSV format.
      --json           The report will be in JSON format.
  -P, --pretty         For JSON reports output will be pretty printed.
  -r, --report <FILE>  The report filename (default stdout).
  -A, --append         Append to the report file, otherwise overwrite.
  -h, --help           Print help
```

#### `admin` commands.

Here is an overview of the administation commands.

```
$ \weather admin
The weather data administration tool.

Usage: weather admin <COMMAND>

Commands:
  init      Initialize the weather data database.
  drop      Delete the existing database schema.
  migrate   Migrate DarkSky archives to internal weather history.
  reload    Reload database weather history for locations.
  show      Show information about the weather data backend components.
  uscities  Administer the US Cities database.
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
Help for subcommands is also available.

```
weather admin init -h
Initialize the weather data database.

Usage: weather admin init [OPTIONS]

Options:
      --hybrid             Configure the database to use archives for history data.
      --document           Configure the database to use JSON for history data.
      --compress           The JSON history data will be compressed in the database.
      --normalized         Configure the database to be fully relational (default).
      --threads <THREADS>  The number of threads to use [default: 8]
      --drop               Drops the database before initializing.
      --load               Load the database after initializing.
  -h, --help               Print help
```
