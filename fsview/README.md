# Filesystem Viewer

A Rust based command line interface (CLI) to display information about local folders and files.

## Background

This project started out as a quick tool to check for duplicate files on my development PC. My system is about 7 years old and has been through several system crashes and forced OS upgrades. Like any good horder, data that could be saved was copied to another disk with the intent to go back through and review, save, delete the contents. Well that never happened and after the 7 years I currently have 4 disks with somewhere in the neighborhood of 1.5M files that I would like to clean up.

While this version currently does not have the analysis part built it does provide some fun summary information. The plan over the winter is to add the analysis part of the tools to make it useful for my needs.

## Installation

As with the weather project there really isn't much needed to get things going. I've only been working on Windoz so far and haven't tried compiling and running on Linux. There are some test cases involving symbolic links that should work but you don't know until you try.

### *Building*

You will need to make sure the *toolslib* library is available. Everything should compile when pulled from Github as long as the directory structure is maintained. Review the `Cargo.toml` file if the directory structure differs.

### *Documentation*

Code documentation is somewhat sparse at the moment. If you do build documentation I would recommend using the following `cargo` command:

>`cargo doc --no-deps --document-private-items`

### *`crates.io`*

I did not try to publish any of this code and I'm not sure I ever would for this silly tool.

## Dependecies

Here are a list of dependencies currently being used.

| Crate | Version | Features |
| :--- | :--- | :---: |
| toolslib | 0.2.0 | |
| clap | 3.1.18 | derive |
| chrono | 0.4.22 | |
| log | 0.4 | |
| logrs | 1.1 | |
| rusqlite | 0.28.0 | bundled |
| serde | 1.0 | derive |
| serde_yaml | 0.9 | |
| thousands | 0.2 | |

## IDE Setup

I moved away from using `CLion` and have primarily been using `VsCode`. I was happy to find out that a `VsCode` workspace could be created which allowed the `toolslib` libary to act like it was part of a Rust workspace.

In order to have full IDE support for rust the `rust-analyzer` extension had to be installed from the market place. I wasn't a fan of the extension in the beginning but have grown to appreciate it as of late. I'm still using `CodeLLDB` and it works well for my debug needs.

## Package Overview

The `fsview` package contains both a binary and library.

* The binary, `fsview`, is a CLI supporting commands to load metadata for file system directories and report details about that metadata.
* A library, `fsview`. that contains the domain object and API that interacts with the metadata persistence layer.

### `fsview` Binary

The binary includes a `cli` module that uses `clap` to define the arguments for the various commands. Here's an overview of the available commands.

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
    -n, --name       List folder contents that match a folders filename
    -p, --path       List the contents of a folder by its pathname
    -r, --root       List the contents of the root folder(s)
    -i, --info       Show a summary of the collected file system information (default)
    -P, --prob       Show a list of files that had an error when loading
    -S, --sum        Show a summary of the files, folders, and size of each folder
    -R, --recurse    Recursively follow a folder structure
    -h, --help       Print help information
```

#### `cli` Module

The `cli` module is built using `clap`. Once agains I am really impressed with the command building API. As with the `weather` project the cli is built using the `#derive` code markup. The ability making argument dependencies, such as `--append` only being allowed with `--log`, is trivial. No code needed to further check argument validity. Nice!

### `fsview` Library

The `fsview` library consist of three (3) modules.

* The `domain` module is what it sounds like, the fsview domain objects and API. The cli uses the domains `Session` object to load and access file system metadata.
* The `filesys` module contains the internal API used by the `domain` to collect local filesystem metadata.
* The `db` module contains the internal API used by the `domain` to save and query the collected filesystem metadata.

#### `domain` Module

The `domain` module contains the public API and objects that front end access to file system information. So it's the middle layer between client and data persistence.

Most of the heavy lifting is loading a folders metadata and converting the results of a query into hierarchical folder information.

#### `filesys` Module

The `filesys` module contains the internal API that traverses a directory collecting metadata. It abstacts the file system `stat` information into various metadata containers. The domain uses the metadata objects to load the database with information for the folder hierarchy.

#### `db` Module

Originally I thought the tool could be stateless however I soon realized that was not going to work. It can take upwards to 20 minutes to load the 500k+ folders and 2.6M files across the five (5) disks I have.

I was not looking forward to requiring some database like Postgres or Mongo to be installed in order to have quicker access to the metadata. Imagine my delight when I discovered there is a SQLite crate available, `rusqlite`, that does not require an external server to be installed.

SQL such as the schema initialization and queries are externalized into separate files contained in a separate folder. The code uses the standard `include_str!` macro to load them into string constants at compile time. The Rust documentation states the provided path is platform specific however the `include_str!` macros use a Linux relative path and it is currently working on Windoz.

The SQL database engine is reasonably fast and so far it has been reliable. The database file itself requires about 1.5 Gib of disk space after loading the metadata from my system. I suspect performance of Postgres or Mongo would be better due to caching on the server however when you consider the CLI database access is essentially stateless, performance right now is good enough. It's when you ask for something like `fsview list --name src -SR` that can take over a minute to complete which is a bit of a concern. I'm not a SQL engineer so I'm sure there is a better way to query but when you consider it contains ~120k folder summaries I think it should hurt a bit.
