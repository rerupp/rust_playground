# `cli` Package

The `cli` module produces the CLI mainline and commands that allow duplicate filename metadata to be loaded and reported.

### CLI Mainline

The mainline is responsible for initializing components, parsing command line arguments, and executing the commands. CommCnds are located in the `cli` module.

Here's an overview of the available commands.

```
c:\ fsview
fsview
A collection of tools that provide information about folders and files in a filesystem

USAGE:
    fsview [OPTIONS] [SUBCOMMAND]

OPTIONS:
        --db <DB>      The name of the database that will be used
        --log <LOG>    The filename logging output will be written into
    -a, --append       Append to the log file, otherwise overwrite
    -v, --verbosity    Logging verbosity level (once=INFO, twice=DEBUG, thrice=TRACE)
    -h, --help         Print help information

SUBCOMMANDS:
    dups    Initializes or reports on duplicate files
    help    Print this message or the help of the given subcommand(s)
    init    Initialize the database schema
    list    Lists folder content and metadata
    load    Loads database with folder metadata
```

The `list` subcommand supports the metadata reporting capabilities.

```
c:\ fsview list -h
fsview-list
Lists folder content and metadata

USAGE:
    fsview list [OPTIONS] [FOLDER]

ARGS:
    <FOLDER>    The folder path or folder name to list

OPTIONS:
    -n, --name          List folder contents that match a folders filename
    -p, --path          List the contents of a folder by its pathname
    -r, --root          List the contents of the root folder(s)
    -i, --info          Show a summary of the collected file system information (default)
    -P, --prob          Show a list of files that had an error when loading
    -D, --details       Show the details of files, folders, and disk space used
    -S, --sum           Show a summary of the files, folders, and size of each folder
    -R, --recurse       Recursively follow a folder structure
        --out <FILE>    The report file pathname
    -a, --append        Append to the log file, otherwise overwrite
    -h, --help          Print help information
 ```

The `dups` subcommand is specific to duplicate filenames. It is used to initialize the duplicate filename metadata. It is also used to generate reports about the duplicate filename data.

```
c:\ fsview dups -h
fsview-dups
Initializes or reports on duplicate files

USAGE:
    fsview dups [OPTIONS]

OPTIONS:
    -i, --init          Initialize the file duplicates metadata
    -r, --report        Generate a report of duplicate files and directories
    -m, --matches       Only include file matches when generating a report
    -n, --none          Only include files that did not match when generating a report
    -s, --summary       Summarize the duplicate files metadata (default)
        --out <FILE>    The report file pathname
    -a, --append        Append to the log file, otherwise overwrite
    -h, --help          Print help information
 ```
