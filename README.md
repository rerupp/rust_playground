# RUST Stuff
This repository has (or will have) a collection of RUST examples and utilities.

## Why Rust?

Back in May 2022 I came across an article that discussed the Linux communities decision to allow Rust to be used within the kernel. I had read several articles on the language and was somewhat surprised due to how (relatively) new the language was.

I spent a week or so going through *The Rust Programming Language* and came away wanting to explore it more. If I could come up with some type of project I could take her out for a spin so to speak.

## The weather Project

Several years ago I wanted to compare historical weather trends for areas on the west coast. The outcome of that was a `Python` project supporting a command line interface, GUI interface, and a simple REST based end point. The implementation and gory details can be reviewed in my `weather` project on GitHub.

I decided building a CLI in Rust that basically emulates the was CLI in `Python` would be a fun journey. Having a reference implementation would allow me to try and duplicate command output. If I really needed to find out how I implemented something I could crank up `PyCharm` and take a look.

## A quick Rust review

The first month or so was pretty frustrating. Coming from a C++, C#, and Java background, thinking outside object the model architecture those languages emloy was somewhat difficult. Almost the opposite of moving from Fortran and C to an object oriented architecture.

### Things I like (today)

* The absence of exceptions in code. I think that was one of the Rustisms I thought "yeah, right" but I'm a believer now. Thank goodness for the syntactical surgar the Rust compiler team has in place otherwise it would be a PITA.
* The data is immutable by default paradigm is okay. It supports not having to build out infrastructure allowing beans to be readonly like is so prevalent in C# and Java. Directly referencing data in a bean is okay because you can't change it unless someone says it's okay.
* The borrow or take ownership of data seemed horrible at first but now I've moslty grown to like it.
* The private or public protection model, both at the module and structure level. It doesn't seem overly robust initially but it does fullfill most circumstances. If not then I'm thinking the design or implementation of the project should probably be reviewed.
* Having the `cargo` tool to do tasks like compile, execute, and produce code documentation. I don't need to include learn a whole new language syntax such as `make`, `mvn`, or `gradle`.
* The Rust community seems pretty engaged right now. I was delighted to see modules to support command line parsing, CSV and JSON processing, Date/Time, and ZIP archives.
* It seems pretty darn fast. Timings for *debug* code is about the same overall as the `Python` implementation. The surprise was a *release* build is about 1/10 the overall time in general. As an example 5 years of reporting weather history on `Python` takes about 10 seconds while a Rust release build is closer to 1 second.

### Things I don't care for (today)

* The language seem verbose to me. In some of the cases I came across (okay, I might come back in a year and go duh) a variable had to be declated in order to use it in a following function call.
* In many cases the compiler generates errors even though it seems to know what I want to do. It goes so far to provide hints as to how my code should look however it is assuming something else. I'm sure this is related to the data ownership model and lifetimes but still.
* Compile time is really, really, slow particularly for release builds. Yeah I know there's a lot going on but still a build after clean can be painful. On Windoz it can take upwards to 2 minutes doing a fresh build. I'm sure antivirus is playing a part but it would be painful on a fairly large project.
* The `CLion` and `VsCode` IDE need work when dealing with refactoring and include documentation. Refactoring a function argument should also update the associated documentation. I know this isn't a Rust issue however it would be a PITA if you were reviewing other developers code and making sure to catch issues like this.

### Things I'm not sure about.

* Lifetimes... I understand the need however it really seems something the compiler could understand and not tell me what I need to do in code because it is assuming something else.
* The syntax surrounding how lifetimes and types are defined seems somewhat thrown on top of the language and verbose. There are numerous areas where a value needed to be assigned before it could be used in a function call. Seems wrong but it is what it is.
* I like the idea of having unit tests next to code, and not having a separate source tree containing them. I'm thinking Rust will require more thought about how to structure code in order to test thoroughly. I read a couple of articles about how to fake mock objects in order to test but I'm thinking it will be difficult to reach a 90% test coverage level mandated by many backend enterprise solutions.
