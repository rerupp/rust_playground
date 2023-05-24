# Function trace logger

A Rust procedural attribute macro that provides the ability to trace log function entry points. The idea behind this package is to log function entry points. I've built similar functionality in *Java* using *AspectJ* and it's come in handy being able to track program flow for backend systems.

The original thoughts were to provide functional similar weaving in *around advice* however I settled on simply logging a function entry point. There are several examples of how to use *syn* and code snipets to explore the abstract syntax tree presents and associated *syn* `Item`.

## Usage

The trace macro requires the `Trace` logging is set for the function path. This might change in the future but that's life right now.

Here's a contrived example of using the attribute.

```text
    mod example {
        #[trace]
        fn greet(name: &str) {
            println!("Hello {name}!!!")
        }
        pub struct Struct(String)
        impl Struct {
            #[trace]
            pub fn hello(&self) {
                greet(&self.0);
            }
        }
    }
```

The logfile output would have the following lines if trace level logging was enabled for the `example` module.

```text
example::Struct.hello - Enter
example::greet - Enter
```

### *Documentation*

Code documentation is somewhat sparse at the moment. If you do build documentation I would recommend using the following `cargo` command:

>`cargo doc --no-deps --document-private-items`

## Implementing *around* advice

I think it would be pretty straight forward to provide something like *around advice*. The idea would be to replace the function with an inner function that would be called. This would allow something like a `Result<T>` to be caught and not have to grok the entire function to catch all exit points.

This would allow capturing function execution times for particular areas of code. The same information can be captured through the log files however it will require all functions to include the trace attribute.

## Rust standalone `struct` functions

As long as the function being traced is from an instance, the full path to the function is pretty straight forward. It becomes a **lot** more work if you want the full path to a `struct` standalone function. The issue really comes about from the abstract syntax tree passed in from the compiler. A `struct` standalone function signature looks just like a plain old function.

While I didn't try this, the approach I would start with is to add a new attribute that would be added to the `impl`. You would continue to add attributes to functions you want to trace just it is done currently. The new attribute would walk the `impl` looking for standalone functions that have a trace attribute. When a function is found the new attribute would insert trace code with a pathname that would include the structure name and remove the existing trace attribute. Instance functions would continue to be called as part of the compile process.

### *`crates.io`*

I did not try to publish any of this code and I'm not sure I ever would for this silly tool.

## Dependecies

Here are a list of dependencies currently being used.

| Crate | Version | Features |
| :--- | :--- | :---: |
| syn | 1.0 | full, extra-traits |
| quote | 1.0 |  |
| proc-macro2 | 1.0 | |
| log | 0.4 | |

## dev Dependencies

| Crate | Version | Features |
| :--- | :--- | :---: |
| env_logger | 0.9 | |
