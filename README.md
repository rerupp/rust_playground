# RUST Stuff

This repository has a collection of RUST examples and utilities.

## Why Rust?

Back in May 2022 I came across an article that discussed the Linux communities decision to allow
Rust to be used within the kernel. I had read several articles on the language and was somewhat
surprised due to how (relatively) new the language was.

I spent a week or so going through *The Rust Programming Language* and came away wanting to explore
it more. If I could come up with some type of project I could take her out for a spin so to speak.

## Projects in this repository

Originally this started out with just an implementation of the weather project however as time went
by this past year several other projects were added.

### The weather Project

Several years ago I wanted to compare historical weather trends for areas on the west coast. The
outcome of that was a `Python` project supporting a command line interface, GUI interface, and a
simple REST based end point. The implementation and gory details can be reviewed in my `weather`
project on GitHub.

I decided building a CLI in Rust that basically emulates the was CLI in `Python` would be a fun
journey. Having a reference implementation would allow me to try and duplicate command output. If I
really needed to find out how I implemented something I could crank up `PyCharm` and take a look.

### The fsview Project

There's more detail within the crate `README.md` however from a mile high view this project will
scan disk folders and then find duplicate files. It is currently implemented with a CLI interface.

### The toolslib Project

This is a collection of tools and utilities that are common to both the *fsview* and *weather*
projects. Nothing fancy just a place to store common code.

### The trace Project

This is a `proc` macro library. It allows you to adorn functions and `struct` instance methods with
annotations to log entry into the function or method. I was hoping to have something like
Java `Aspects` at compile time however to provide `around` capabilities was going to take to much
time.

## A quick Rust review

The first year or so was pretty frustrating. Coming from a C++, C#, and Java background, thinking
outside the object model architecture those languages employ was difficult. Almost the opposite of
moving from Fortran and C to an object oriented architecture.

Now that it has been a little over 2 years I really like working with this language. I feel like 
the language still needs to mature a bit but the community is active. While asychronous function 
calls are supported in the language the runtime is left to `crates.io`. It would really be 
nice if there was an official library version

References to objects and their lifetimes continues to be a source of frustration for me. I  
understand why they are needed and how to use them however getting into the lifetime muck can
be a headache. That said the standard library has object counting containers such as `Rc<T>` that 
provide an alternative. Yes there is overhead using them but we did that for years using `stl` 
smart pointers.


### Things I like

* The absence of exceptions in code. I think that was one of the Rustisms I thought "yeah, right"
  but I'm a believer now. Thank goodness for the syntactical surgar the Rust compiler team has in
  place otherwise it would be a PITA.
* The data is immutable by default paradigm is okay. It supports not having to build out
  infrastructure allowing beans to be readonly like is so prevalent in C# and Java. Directly
  referencing data in a bean is okay because you can't change it unless someone says it's okay.
* Having the `cargo` tool to do tasks like compile, execute, and produce code documentation. I don't
  need to learn a whole new language syntax such as `make`, `mvn`, or `gradle`.

### Things I don't care for (today)

* Compile time. I think there are plans to address this but it is annoying right now. For a large
  project I can envision complie times that could take a long time to complete.

## Rust notes

This is a collection of things I seem to keep looking up when I come across issues.

* `CARGO_LOG=cargo::core::compiler::fingerprint=info` will instruct `cargo` to log dependencies to
  help you understand why rebuilds are occuring. This helped identify issues with `BitDefender`
  muking with file timestamps and causing libraries to be rebuilt.
