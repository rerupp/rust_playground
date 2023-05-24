# Weather Data CLI

### The `main` program

The CLI mainline is responsible for intialization of libraries and domain, parsing the command line, and executing the commands. It also will setup the logger to emit `INFO`, `DEBUG`, and `TRACE` level logging.

### The `cli` Module

The CLI is built on top of the `clap` crate. All of the sub-commands support producing report output in the form of plain text, JSON, or CSV formatted text.

Unlike `Python` this implementation only supports reporting weather data. It does not support adding either locations or weather data.

As with the `Python` implementation, the weather executable consists of subcommands to create various reports. If a subcommand is not entered a help overview is provided.

```
$ weather.exe
The CLI commands available for weather data

USAGE:
    weather.exe [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -d, --directory <DATA_DIR>    The directory pathname containing weather data
        --log <LOG>               The filename logging output will be written into
    -a, --append                  Append to the log file, otherwise overwrite
    -v, --verbosity               Logging verbosity level (once=INFO, twice=DEBUG, thrice=TRACE)
    -h, --help                    Print help information
    -V, --version                 Print version information

SUBCOMMANDS:
    help    Print this message or the help of the given subcommand(s)
    lh      List weather data, by date, available by location
    ll      Show weather data locations
    ls      Show a summary of weather data available by location
    rh      Generate a weather data report for a location
```

Help for subcommands are also available.

```
$ weather.exe rh -h
Generate a weather data report for a location

USAGE:
    weather.exe rh [OPTIONS] <LOCATION> <START> [ENDS]

ARGS:
    <LOCATION>    The location used for the details report
    <START>       The starting date for the report
    <ENDS>        The ending date for the report

OPTIONS:
    -t, --temp               Include daily temperatures in the report (default)
    -c, --cnd                Include daily conditions in the report
    -m, --max                Include min/max temperatures in the report
    -s, --sum                Include a summary of the weather in the report
    -a, --all                Include all data in the generated report
        --text               The output will be plain Text (default)
        --csv                The output will be in CSV format
        --json               The output will be in JSON format
    -f, --file <FILENAME>    The report output file pathname, if not provided output is directed to
                             *stdout*
    -p, --pretty             This flag is specific to JSON reports and indicates reports should be
                             more human readable
    -h, --help               Print help information
```

Some of the command produce slightly different formatted output but it was something I had been wanting to do in `Python` for some time.

#### `clap` Goodness

I would like to provide a quick "this is what I like about" the command parser.

* The `clap` `#derive` syntax is used to implement the subcommand parsing. It is one of the easiest API's I've used to manage complex command option relatsionships. If you take a look at the `rh` subcommand you will see a pretty complex set of flags and flag dependencies. I thought at one point I would need to do some post processing of arguments but `clap` `#derive` was able to manage it. Nice!

* It is easy to install custom parsers and custom validators for option arguments. Most command lines frameworks facilitate this but it was nicely intergrated.
