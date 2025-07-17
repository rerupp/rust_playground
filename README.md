# The `Rust` Playground

The repository contains a collection of different `Rust` based programs and libraries.

## Why Rust?

Back in May 2022 I came across an article that discussed the Linux communities decision to allow
Rust to be used within the kernel. I had read several articles on the language and was somewhat
surprised due to how (relatively) new the language was.

I spent a week or so going through *The Rust Programming Language* and came away wanting to explore
it more. If I could come up with some type of project I could take her out for a spin so to speak.

## Repository Projects

### Historical Weather Data

The repository originally started out with a `weather` project which was a clone of the `Python` 
project command line. The implementation and gory details are still available in my `weather`
project on GitHub.

The project now includes a `Rust` based terminal UI and a `Python` based GUI.

#### The `weather` Project

The `weather` project contains the `Rust` based implementation of the `Python` CLI, the terminal 
UI, and the library used to read and write historical weather data. The readme file in the 
project contains the gory details of how to bootstrap and build the project.

This project should be built before using the `Python` GUI in order to bootstrap the historical 
weather data directory.

#### The `py_weather` Project

This project contains the `Python` GUI and `PyO3` weather data bindings. At some point the
GIT repository should be renamed to reflect that it is not just a `Rust` project but that is for
another day.

### The `fsview` Project

There's more detail within the crate `README.md` however from a mile high view this project will
scan disk folders and then find duplicate files. It is currently implemented with a CLI interface.

### The `toolslib` Project

This is a collection of tools and utilities that are common to both the `fsview` and `weather`
projects. Nothing fancy just a place to store common code.

### The `trace` Project

This is a `proc` macro library. It allows you to adorn functions and `struct` instance methods with
annotations to log entry into the function or method. I was hoping to have something like
Java `Aspects` at compile time however to provide `around` capabilities was going to take to much
time.

## A quick Rust review

The first year or so was pretty frustrating. Coming from a C++, C#, and Java background building
as a "Rustacean" was certainly a change. Almost the opposite of moving from languages like
`Fortran` and `C` to an object model architecture.

Now that I've got several years playing with `Rust`, I really like the language. Object lifetimes,
while sometimes frustrating, are much easier to work through and follow. While still a young
language it continues to mature and move forward. One of the most exciting growths is the
progress being made towards asynchronous functions parity in the `std` library. 

The development environment is also moving forward. The compiler is faster now and better at
providing diagnostics for programming errors. When panics occur messages are more clear and
precise, particularly for common errors. 

### Things I like

* The absence of exceptions in code. That was one of the `Rust` paradigms I thought "yeah, right"
  but now I'm a believer.
* The "data is immutable by default" paradigm. It facilitates read-only access to bean-like  
  `struct`s by simply not marking them mutable. You don't need to introduce a read-only interface 
  or add code metadata to access attributes.
* Code refactoring. I've gone through some pretty major re-writes to the weather front-end and 
  backend library. Assuming no logic errors are made, if it compiles it works. Really 
  confidence inspiring.
* `Rust` executables are quick. Enough said.
* The `Rust` community crate registry. Packages such as `clap`, `chrono`, `serde`, `termio` and 
  `rusqlite` are surprisingly mature.
* The `cargo` tool. Using it perform tasks like add dependencies, compile, execute, and produce 
  code documentation means I didn't need to learn a whole new language syntax such as with `make`, 
  `mvn`, or `gradle`.

### Things I don't care for (today)

* Compile time. There is progress being made on this front but compiles are still a little slow. 
  For a really large project I could see this being an issue. On the other side the structure of 
  a large project could mitigate some of my concerns.

## Rust notes

This is a collection of things I seem to keep looking up when I come across issues.

* `CARGO_LOG=cargo::core::compiler::fingerprint=info` will instruct `cargo` to log dependencies to
  help you understand why rebuilds are occurring. This helped identify issues with `BitDefender`
  messing with file timestamps and causing libraries to be rebuilt.
