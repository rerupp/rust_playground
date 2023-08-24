# Weather Data CLI

### The `main` program

The CLI mainline is responsible for intialization of libraries and domain, parsing the command line, and executing the commands. It also will setup the logger to emit `INFO`, `DEBUG`, and `TRACE` level logging.

### The `cli` Module

The CLI is built on top of the `clap` crate. All of the sub-commands support producing report output in the form of plain text, JSON, or CSV formatted text.

Unlike `Python` this implementation only supports reporting weather data. It does not support adding either locations or weather data.

As with the `Python` implementation, the weather executable consists of subcommands to create various reports. If a subcommand is not entered a help overview is provided.

```
$ weather
The CLI commands available for weather data

Usage: weather [OPTIONS] [COMMAND]

Commands:
  ll    Show weather data locations
  ls    Show a summary of weather data available by location
  lh    List weather data, by date, available by location
  rh    Generate a weather data report for a location
  help  Print this message or the help of the given subcommand(s)

Options:
  -d, --dir <DATA_DIR>  The directory pathname containing weather data
      --db              Use a database for weather data
  -l, --log <LOG>       The filename logging output will be written into
  -a, --append          Append to the log file, otherwise overwrite
  -v...                 Logging verbosity level (once=INFO, twice=DEBUG, thrice=TRACE)
  -h, --help            Print help (see more with '--help')
  -V, --version         Print version
  ```

Help for subcommands are also available.

```
$ weather rh -h
Generate a weather data report for a location

Usage: weather rh [OPTIONS] <LOCATION> <START> [ENDS]

Arguments:
  <LOCATION>  The location used for the details report
  <START>     The starting date for the report
  [ENDS]      The ending date for the report

Options:
  -t, --temp           Include daily temperatures in the report (default)
  -c, --cnd            Include daily conditions in the report
  -m, --max            Include min/max temperatures in the report
  -s, --sum            Include a summary of the weather in the report
  -a, --all            Include all data in the generated report
      --text           The output will be plain Text (default)
      --csv            The output will be in CSV format
      --json           The output will be in JSON format
  -r, --report <FILE>  The name of a file report output will be written too
  -A, --append         Append to the log file, otherwise overwrite
  -p, --pretty         For JSON reports have content be more human readable
  -h, --help           Print help (see more with '--help')
```

Some of the command produce slightly different formatted output but it was something I had been wanting to do in `Python` for some time.

#### `clap` Goodness

Here's what I liked about the command parser.

* The `#derive` syntax was used to implement the sub-command parsing. It is one of the easiest API's I've used to manage complex command option relatsionships. If you take a look at the `rh` subcommand you will see a pretty complex set of flags and flag dependencies. I thought at one point I would need to do some post processing of arguments but `clap` `#derive` was able to manage it. Nice!

* It is easy to install custom parsers and custom validators for option arguments. Most command lines frameworks facilitate this but it was nicely intergrated.

When I first created the CLI, `#derive` made sense to me. I'm thinking differently now after upgrading to the latest release. It was not fun trying to figure what broke in the derive attributes across the sub-commands. As I got up to speed with the newer changes in `clap` and fixed the derive issues I decided to use the programming API for the adminstration CLI. I think it was a good move.