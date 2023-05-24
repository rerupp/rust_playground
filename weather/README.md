# Weather Data

A Rust base command line interface (CLI) for displaying weather history.

## Background
This project started as a way to familiarize myself with Rust. I was looking for some project to build and decided emulating the `Python` weather project I built several years ago would be fun.

Even though the *Dark Sky* data API is no longer available, I have lots of data that can be used and the `Python` project would be a handy reference implementation I could always look at. The performance of `Python` was pretty reasonable and I thought it would be fun to compare apple and oranges.

## Project Structure
The project is actually a Rust workspace consisting of a binary CLI crate and a library crate. It has a dependency on the `toolslib` crate.

The `Cargo.toml` at the workspace level contains common dependencies for both CLI and library crates.

### `cli` Directory
This directory contains the source code for the CLI mainline.

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

Here are a list of dependencies currently being used.

| Crate | Version | Features |
| :--- | :--- | :----: |
| clap | 3.1.18 | derive |
| serde | 1.0.137 | derive | 
| serde_json | 1.0.81 | preserve_order |
| zip | 0.6.2 | |
| thousands | 0.2.0 | |
| chrono | 0.4 | |
| chrono_tz | 0.8 | serde |
| csv | 1.1 | |
| log | 0,4 | |

## IDE Setup
Here are a couple of notes on setting up the VsCode IDE to run and debug. I did have to install the Rust plugin from the *JetBrain* marketplace but that's it.

### *VsCode*
In order to have full IDE support for rust the `rust-analyzer` extension had to be installed. This needs to be done for both Windoz and Linux platforms. VsCode took some time to understand the Rust source but after that it was useful.

#### *Linux* OS
In order to debug code I had to install more tools. Initially I used the `CodeLLDB` extension but then settled on the Microsoft cpp developer tools. Both worked I just settled on the MS blessed tool set.

#### *Windoz* OS
I installed Visual Studio Community before installing Rust so I assumed debug support would be available but alas no. I Installed `CodeLLDB` and it works well for my usage. My old *AVG* antivirus kept detecting a generic virus in the DLL but after switching to another vendor it works fine.

