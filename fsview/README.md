# Filesystem Viewer

A Rust based command line interface (CLI) to display information about folders and files.

## Background

This project started out as a quick tool to check for duplicate files on my development PC. My desktop is about 7 years old, has been through several disk crashes (Intel RAID sucks), along with the forced Windoz 10 upgrade. It's time to replace it and like any good horder, data that could be recovered was copied to another disk with the intent to go back through and review, save, delete the contents. Well that never happened and after the 7 years I currently have 4 disks with somewhere in the neighborhood of 1.5M files that I would like to clean up.

The CLI implementation is functional and mostly complete. Commands allow filesystem metadata to be loaded into a SQLite3 database and reports exist that show:

* All the folders that had matching filenames.
* The folders that have matching files.
* The folders that did not have file matches.

## Installation

As with the weather project there really isn't much needed to get things going. I've been working on Windoz and WSL 2 to verify code on mltiple platforms. At some point I should build it on a real Linux OS but so far WSL 2 with Ubuntu is everybit as functional as Fedora 32 I've been using. There are some test that are Windoz specific but they are isolated into a separate module.

### *Building*

You will need to make sure the *toolslib* library is available. Everything should compile when pulled from Github as long as the directory structure is maintained. Review the `Cargo.toml` file if the directory structure differs.

### *Documentation*

Code documentation is available however I would not consider it to be well documented code. If you do build documentation I would recommend using the following `cargo` command:

>`cargo doc --workspace --no-deps --document-private-items`

### *`crates.io`*

I did not try to publish any of this code and I'm not sure I ever would for this silly tool.

## Dependecies

Here are a list of dependencies currently being used.

| Crate | Version | Features | Module |
| :--- | :--- | :---: | :--: |
| clap | 3.2 | derive | cli |
| chrono | 0.4 | | workspace |
| log | 0.4 | | workspace |
| log4rs | 1.2 | | cli |
| rusqlite | 0.28 | bundled | fslib |
| serde | 1.0 | derive | workspace |
| serde_yaml | 0.9 | | fslib |
| toolslib | latest | | workspace |

## IDE Setup

I moved away from using `CLion` and have primarily been using `VsCode`. I was happy to find out that a `VsCode` workspace could be created which allowed the `toolslib` libary to act like it was part of a Rust workspace. I'm not sure you can do the with `CLion` and it seems that's by design.

In order to have full IDE support for rust the `rust-analyzer` extension had to be installed from the market place. I wasn't a fan of the extension in the beginning but have grown to appreciate it as of late. I'm still using `CodeLLDB` and it works well for my debug needs.

## Workspace Overview

The `fsview` workspace consists of a CLI mainline and a supporting library package.

### `cli` Module

The `cli` module produces a cli binary named `fsview`. The binary has commands to load filesystem metadata and report duplicate file details.


### `lib` Module

The `lib` module provides the backend support for the CLI. It provides the API and objects used to initialize and report the duplicate file metadata.
