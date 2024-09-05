# Weather Data

The weather data project collects and displays historical weather data for some location
based on its latitude and longitude. Primarily it is for cities in the US but any location
in the world can be set up and used.

## Why Am I Doing This???

This project started as a way to familiarize myself with Rust. I was looking for some project to
build and decided emulating the `Python` weather project I built several years ago would be fun.
The performance of the `Python` apps was reasonable and I thought it would be fun to compare
apples and oranges.

### Background

The original `Python` implementation was built on top of the *DarkSky* weather data `REST` API.
Even though *DarkSky* API is no longer publically available, I had *DarkSky* historical data from 
the `Python` project that was. Implementing a CLI in `Rust` similar to the `Python` version
would allow a pretty deep dive into `Rust` and its ecosystem.

### Initial Release

Prior to the October 2023 version, the backend data was fully *DarkSky* based. The implementation
used the same data archives as the `Python` project. Although there were some minor differences
it did reproduce the `Phyton` CLI. The Rust `Zip` archive debug binary was 3-4 times faster than
`Python` and relase binaries where 6-8+ times faster.

### October 2023 Release

The one part of the `Python` API I did not try to implement in the initial version was adding
weather data. After doing some research I found an API at
[Visual Crossings](https://www.visualcrossing.com/) that would work. I could sign up for free and
collect up to 1000 histories per day. I was pleasantly surprised with the available data and
performance of their [Timeline Weather API](https://www.visualcrossing.com/resources/documentation/weather-api/timeline-weather-api/).
For the needs of this project it is perfect. I can collect months of weather history in seconds 
(**thank you Visual Crossing**).

I wanted to avoid storing history documents from *Visual Crossing* similar to what was done with
the original *DarkSky* documents. After reviewing the historical data being used a new 
`JSON` document structure was created. An **admin** command was added that *migrates* 
existing *DarkSky* documents to the new format. The backend archive data store has been
retained, only the document contents have changed. Unfortunately this breaks
compatability with the `Phython` version.

In order to support calling the *Timeline API* a `weather.toml` configuration file was added. 
Add the following line to the file replacing the ellipse with your *Timeline* API key.

    key = "..."

When the file is present in the directory where you run the *weather* application, it will be used
to include the *Timeline* API key. If the configuration file is not present the process environment
will be searched for a `VISUAL_CROSSING_KEY` variable holding the key. I did not include an 
option to specify it on the command line.

### September 2024 Release

This release has brought many changes to weather data. The *admin* command line executable was
removed and added into the main CLI. Locations and weather history can be both be added to weather 
data. Since writing a `GUI` in Rust is still a PITA on windows I decided to build out a 
terminal UI based on the `ratatui` and `crossterm` crates. The common parts of the TUI is in a 
[terminal UI](termui/README.md) library under the `weather` directory. It will probably be moved 
peer to `toolslib` at some point in the future. 

I started to have problems using VS Code in the project. The editor became pretty sluggish and
had difficulties staying up to date. I moved over to JetBrains `RustRover` and have been mostly
pleased. There are some pretty annoying issues with it however it has helped a lot.

Here's what bugs me.

- One of the most annoying issues with `RustRover` is compiler errors. Too many times when 
  compiling code, the build window shows the build failed but there is nothing in the output window.
  I continually need to drop out to the command line, run a `cargo build` then scroll throught 
  the output to find errors. Arg!!!
- Debugging is problematic. When debugging simple unit tests, break points are missed 
  forcing you to step into functions. When stepping into functions, more times than not, you 
  wind up in an assembly code window.

Most of the CLI source code has either been refactored or removed. A new text report manager
was added to `toolslib` allowing a common report to be used by the `rh` command and the TUI.
I drank the Rust coolade and started taking advantage of the `Result` and `ControlFlow` constructs.
After 2 years with Rust I have gained a lot of respect for the language. There have been some
pretty brutal refactoring sessions and I have a lot of confidence in the language facilities to 
help me through the changes.

## Project Structure

The weather project is a `cargo` based workspace consisting of the CLI mainline and supporting 
libraries. It has a dependency on the `toolslib` crate.

### `cli` Directory

This directory contains the source code for the CLI mainline.

### `lib` Directory

This directory contains the backend implementation of the weather domain.

### `termui` Directory

This directory contains the low level components used to build the CLI UI interface.

## Getting Started

There really isn't much to do in order to get things going. Follow the Rust install 
directions and everything else is straight forward.

Here are the steps to get started (from the `weather` directory).
```
$ cargo build
$ mkdir weather_data
$ target\debug\weather admin uscities --load=resources\uscities.csv
$ target\debug\weather tui 
```
From the main window press `ALT-N`, followed by `ALT-S`, followed by `ALT-U`. This will bring
up the US Cities search dialog allowing you to add a location. Once you have a location added 
you can press `ENTER` while on the location to bring up a context menu that will allow you to add 
or report weather history.

## Build Environment

I haven't built on WSL2 for a while but here's information about the toolchain on Windoz.

```
$ rustup show
Default host: x86_64-pc-windows-msvc
rustup home:  C:\Users\rncru\.rustup

stable-x86_64-pc-windows-msvc (default)
rustc 1.80.1 (3f5fd8dd4 2024-08-06)
```

### *Documentation*

If you're going to build documentation I would suggest using the following `cargo` command.

```
cargo doc --workspace --no-deps --document-private-items
```

## Dependencies

Here are a list of workspace dependencies.

| Crate      | Version |       Features        |
|:-----------|:--------|:---------------------:|
| chrono     | 0.4     |         serde         |
| chrono_tz  | 0.9     |         serde         |
| log        | 0.4     |                       |
| rusqlite   | 0.30    | blob, bundled, chrono |
| serde      | 1       |        derive         | 
| serde_json | 1       |    preserve_order     |
| toml       | 0.8     |    preserve_order     |
| crossterm  | 0.28.1  |                       |
| ratatui    | 0.28    |  all-widgets, serde   |
