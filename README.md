
# RUST Stuff
This repository has a collection of RUST examples and utilities.

## Why Rust?
Back in May 2022 I came across an article that discussed the Linux communities decision to allow Rust to be used within the kernel. I had read several articles on the language and was somewhat surprised due to how (relatively) new the language was.

I spent a week or so going through *The Rust Programming Language* and came away wanting to explore it more. If I could come up with some type of project I could take her out for a spin so to speak.

## Projects in this repository
Originally this started out with just an implementation of the weather project however as time went by this past year several other projects were added.

### The weather Project
Several years ago I wanted to compare historical weather trends for areas on the west coast. The outcome of that was a `Python` project supporting a command line interface, GUI interface, and a simple REST based end point. The implementation and gory details can be reviewed in my `weather` project on GitHub.

I decided building a CLI in Rust that basically emulates the was CLI in `Python` would be a fun journey. Having a reference implementation would allow me to try and duplicate command output. If I really needed to find out how I implemented something I could crank up `PyCharm` and take a look.

### The fsview Project
There's more detail within the crate `README.md` however from a mile high view this project will scan disk folders and then find duplicate files. It is currently implemented with a CLI interface.

### The toolslib Project
This is a collection of tools and utilities that are common to both the *fsview* and *weather* projects. Nothing fancy just a place to store common code.

### The trace Project
This is a `proc` macro library. It allows you to adorn functions and `struct` instance methods with annotations to log entry into the function or method. I was hoping to have something like Java `Aspects` at compile time however to provide `around` capabilities was going to take to much time.

## A quick Rust review
The first month or so was pretty frustrating. Coming from a C++, C#, and Java background, thinking outside the object model architecture those languages employ was difficult. Almost the opposite of moving from Fortran and C to an object oriented architecture.

Now after a year playing with the language I've really grown to appreciate it. It's still frustrating but I'm coming to the conclusion I still think back to objects, hieararchy, and models instead of thinking how you could approach it with Rust.

### Things I like
* The absence of exceptions in code. I think that was one of the Rustisms I thought "yeah, right" but I'm a believer now. Thank goodness for the syntactical surgar the Rust compiler team has in place otherwise it would be a PITA.
* The data is immutable by default paradigm is okay. It supports not having to build out infrastructure allowing beans to be readonly like is so prevalent in C# and Java. Directly referencing data in a bean is okay because you can't change it unless someone says it's okay.
* The borrow or take ownership of data seemed horrible at first but now I've moslty grown to like it.
* The protection model being either private or public. This can be hard to get over coming from other languages but it does simplify the design of code. The other nicety is `pub(crate)` or `pub(super)` to lock down functions to the module or crate level. This facilitates access to common functionality in a libary and guarantees it cannot be leaked outside the crate.
* Having the `cargo` tool to do tasks like compile, execute, and produce code documentation. I don't need to learn a whole new language syntax such as `make`, `mvn`, or `gradle`.
* The Rust community seems pretty engaged right now. I was delighted to see modules to support command line parsing, CSV and JSON processing, Date/Time, ZIP archives, SQLite, etc.
* I really like `macro_rules`. Having access to the compiler token tree syntax and parse it is pretty cool. Much better than the `#define` paradigm in C/C++.
* It seems pretty darn fast. Timings for *debug* code is about the same overall as the `Python` implementation. The surprise was a *release* build is about 1/10 the overall time in general. As an example 5 years of reporting weather history on `Python` takes about 10 seconds while a Rust release build is closer to 1 second.

### Things I don't care for (today)
* Compile time. I think there are plans to address this but it is annoying right now. For a large project I can envision complie times that could take a long time to complete.
* In many cases the compiler generates errors even though it seems to know what I want to do. It goes so far to provide hints as to how my code should look however it is assuming something else. 
* The `CLion` and `VsCode` IDE need work when dealing with refactoring and documentation. Refactoring a function argument should also update the associated documentation. I know this isn't a Rust issue however it would be a PITA if you were reviewing other developers code and making sure to catch issues like this.
* Some of the Rust syntax is pretty crazy in my opinion. Things like scoping generics to a particular trait, managing lifetimes, or casting the type of an iterator bring "what the heck is the syntax of that again?" moments for me a lot.

### Things I'm not sure about.
* I like the idea of having unit tests next to code, and not having a separate source tree containing them. I'm thinking Rust will require more thought about how to structure code in order to test thoroughly. I read a couple of articles about how to fake mock objects in order to test but I'm thinking it will be difficult to reach a 90% test coverage level mandated by many backend enterprise solutions.

## Rust notes
This is a collection of things I seem to keep looking up when I come across issues.

* `CARGO_LOG=cargo::core::compiler::fingerprint=info` will instruct `cargo` to log dependencies to help you understand why rebuilds are occuring. This helped identify issues with `BitDefender` muking with file timestamps and causing libraries to be rebuilt.
