# Weather Data

A Rust base command line interface (CLI) for displaying weather history.

## Why Am I Doing This???
This project started as a way to familiarize myself with Rust. I was looking for some project to build and decided emulating the `Python` weather project I built several years ago would be fun. The performance of the `Python` apps was reasonable and I thought it would be fun to compare apples and oranges.

### Background

The original `Python` implementation was built on top of the *DarkSky* weather data `REST` API.Even though *DarkSky* API is no longer available, I do have *DarkSky* historical data from the `Python` project that could be used. Implementing a CLI in `Rust` similar to the `Python` version would allow a pretty deep dive into `Rust` and its ecosystem.

### Initial Implementation

Prior to the October 2023 version, the backend data was fully *DarkSky* based. The implementation used the same data archives from the `Python` project. Although there are some minor differences it did reproduce the `Phyton` CLI. The `Zip` archive debug binary was 3-4 times faster than `Python` and relase binaries where 6-8+ times faster.

### Current Implemenation

The one part of the `Python` API I did not try to implement in the initial version was adding weather data. It was bugging me so I did some research and found an API at [Visual Crossings](https://www.visualcrossing.com/) that would work. I could sign up for free and collect up to 1000 histories per day. I was pleasantly surprised with the available data and performance of their [Timeline Weather API](https://www.visualcrossing.com/resources/documentation/weather-api/timeline-weather-api/). For my needs it is perfect. I can collect months of weather history in seconds (**thank you Visual Crossing**).

I wanted to avoid storing history documents from *Visual Crossing* similar to what I did with *DarkSky*. After reviewing the data being used I created a `JSON` document structure that's mostly based on the data used in history reports. I added an **admin** command to *migrate* existing *DarkSky* documents to the new format. I retained the same `Zip` file naming convention. Unfortunately this breaks compatability with the `Phython` version.

In order to support calling the *Timeline API* I added a `weather.toml` configuration file. Add the following line to the file replacing the ellipse with your *Timeline* API key.

    key = "..."

When the file is present in the directory where you run the *weather* application, it will be used to include the *Timeline* API key. If the configuration file is not present the process environment will be searched for a `VISUAL_CROSSING_KEY` variable being set and use that. I did not include an option to specify it on the command line.

History can be added when using the `Zip` archive backend implementation and when using a *normalized* database configuration. When using the *normalized* database configuration, weather history is added to both the `Zip` archive and database. The `Zip` archive continues to be where weather history should be kept long term. Performance with the new document structure is better than with the previous version.

## Project Structure
The project is a Rust workspace consisting of two binary command lines and a library crate. It has a dependency on the `toolslib` crate.

The `Cargo.toml` at the workspace level contains common dependencies for both CLI and library crates.

### `cli` Directory
This directory contains the source code for the CLI mainline.

### `admin` Directory
This directory contains the source code for the administrative CLI.

### `lib` Directory
This directory contains the backend implementation of the weather domain.

## Installation
There really isn't much to do in order to get things going. Of course you need to install Rust but everything else should be straight forward. As of May 2023 I'm using the latest stable build.

I was delighted to see the code compiled as is for both Windoz and Linux. The initial code was created on Fedora 36 using Jet Brains *CLion* IDE. I basically copied source over to Windoz and did a `cargo build`. Nice!


### *Documentation*
If you're going to build documentation I would suggest using the following `cargo` command.

> `cargo doc --workspace --no-deps --document-private-items`

### *`crates.io`*
I did not try to publish anything and I'm not sure I would for this silly thing.

## Dependencies

Here are a list of workspace dependencies.

| Crate | Version | Features |
| :--- | :--- | :----: |
| chrono | 0.4 | |
| chrono_tz | 0.8 | serde |
| log | 0.4 | |
| rusqlite | 0.29 | blob, bundled, chrono |
| serde | 1.0.137 | derive | 
| serde_json | 1.0.81 | preserve_order |
| toml | 0.7 | preserve_order |

## IDE Setup
Here are a couple of notes on setting up the VsCode IDE to run and debug. I did have to install the Rust plugin from the *JetBrain* marketplace but that's it.

### *VsCode*
In order to have full IDE support for rust the `rust-analyzer` extension had to be installed. This needs to be done for both Windoz and Linux platforms. VsCode took some time to understand the Rust source but after that it was useful.

#### *Linux* OS
In order to debug code I had to install more tools. Initially I used the `CodeLLDB` extension but then settled on the Microsoft cpp developer tools. Both worked I just settled on the MS blessed tool set.

#### *Windoz* OS
I installed Visual Studio Community before installing Rust so I assumed debug support would be available but alas no. I Installed `CodeLLDB` and it works well for my usage. My old *AVG* antivirus kept detecting a generic virus in the DLL but after switching to another vendor it works fine.

