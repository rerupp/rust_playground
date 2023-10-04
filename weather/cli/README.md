# Weather Data CLI

### The `main` program

The CLI mainline is responsible for getting the CLI implementation and parsing the command line arguments. Intialization of the runtime and execution of commands is now delegated to the `cli` module. It also will setup the logger to emit `INFO`, `DEBUG`, and `TRACE` level logging.

### The `cli` Module

The CLI is built on using the `clap` crate. In the previous version the commands were implemented using `#[derive(...)]` macros however I re-wrote the CLI using the program API. This allowed me to get rid of a lot of crap and facilitated separating arguments from implementation. I like the implementation patterns being used. For me reading the API implementing commands and arguments is a lot more consise than `#derive` on top of structures.

 All of the sub-commands support producing report output in the form of plain text, `JSON`, or `CSV` formatted text.

As with the `Python` implementation, the weather executable consists of subcommands to create various reports. If a subcommand is not entered a help overview is provided.

```
$ weather
The weather data command line.

Usage: weather [OPTIONS] <COMMAND>

Commands:
  ll    List the weather data locations that are available.
  ls    List a summary of weather data available by location.
  lh    List the dates of weather history available by location.
  rh    Generate a weather history report for a location.
  ah    Add weather history to a location.
  help  Print this message or the help of the given subcommand(s)

Options:
  -d, --directory <DIR>    The weather data directory pathname.
      --db                 Use a database configuration for weather history.
  -l, --logfile <LOGFILE>  The log filename (DEFAULT stdout).
  -a, --append             Append to the logfile, otherwise overwrite.
  -v, --verbose...         Logging verbosity (once=INFO, twice=DEBUG, +twice=TRACE)
  -h, --help               Print help
  -V, --version            Print version
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

Some of the command produce slightly different formatted output but it was something I had been wanting to do in `Python` for some time.

#### `clap` Goodness

Here's what I liked about the command parser.

* The `Command` API is really a nice implementation. The pattern of having a struct represent a collection of arguments is something I really like. It's trivial for the various commands to include report type arguments and access argument value that have been parsed.

* It is easy to add custom parsers and validators for arguments. Most command line frameworks facilitate this but I particularly like how `clap` has implemented this. You can call a function that validates a directory argument and converts that to a `PathBuf`. When you access the argument can expect it to be a `PathBuf` not a string.
