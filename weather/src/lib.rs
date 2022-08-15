//! A RUST based weather data sample implementation.
//!
//! This implementation is loosely base on a `Python` project I created several years ago. When
//! I started the `Python` project I wanted to chart historical weather information for different
//! areas we were interested in spending the winter. The idea of building a CLI based on the
//! original weather data implementation seemed like a fun journey.
//!
//! # History
//!
//! My intent was to build something to continue with RUST and after going through the various
//! tutorials. The `Python` weather data project is based on the ***Dark Sky*** project data.
//! Unfortunately the API was purchased by Apple and is no longer publicly available (or at least
//! free) but I had collected years of data for a dozen or more sites.
//!
//! # Architecture Overview
//!
//! The crate consists of three modules.
//!
//! * A front end (cli)
//! * The weather data API and data objects (domain)
//! * The API that reads weather data (data).
//!
//! The three (3) modules are loosely coupled. Something has to assemble them (alas no `Spring`
//! framework) so there is a *main* binary that does the work.
//!
//! Here's code that implements a *main*.
//!
//! ```no_run
//! fn main() -> cli::CliResult<()> {
//!     let cmd: cli::Cli = cli::Cli::parse();
//!     let data_api = data::from_pathname(cmd.data_dir())?;
//!     let weather_data = WeatherData::new(data_api);
//!     cli::dispatch(cmd, &weather_data)
//! }
//! ```
//!

/// The command line interface (CLI) to access weather data.
pub mod cli;

/// The data API to read the stored weather data.
pub mod data;

/// The weather data domain connecting the CLI to weather data.
pub mod domain;

/// A collection of objects common to all modules.
pub mod core {
    use std::error;
    use std::fmt;

    /// Defines a unified *Result* definition for all modules.
    pub type WeatherResult<T> = Result<T, Box<dyn error::Error>>;

    /// The `Error` object used by all modules.
    #[derive(Debug, Clone)]
    pub struct WeatherError(pub String);

    /// The default implementation of a weather error.
    impl WeatherError {
        /// Creates an instance of the error.
        ///
        /// # Arguments
        ///
        /// * `msg` - A string slice that describes the error condition.
        pub fn new(msg: &str) -> Box<dyn error::Error> {
            Box::new(WeatherError(msg.to_string()))
        }
    }

    impl fmt::Display for WeatherError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", &self.0)
        }
    }

    impl error::Error for WeatherError {}

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn err_test() {
            let msg = "some error message";
            let result = echo_error(msg);
            assert!(result.is_err());
            assert_eq!(msg, format!("{}", result.unwrap_err()));
        }

        fn echo_error(msg: &str) -> WeatherResult<()> {
            Err(WeatherError::new(msg))
        }
    }
}

/// A utility to track the duration of some operation.
///
/// Yeah, there are a lot of these things out there but this is the type
/// of utility I'm use to so here it is.
mod stopwatch {
    use std::fmt;
    use std::fmt::Formatter;
    use std::time::{Duration, Instant};

    /// The stopwatch data.
    #[derive(Debug)]
    pub struct StopWatch {
        /// When the stop watch was started or `None`.
        start: Option<Instant>,
        /// How long the stopwatch was run or `None`
        duration: Option<Duration>,
    }

    /// How the stopwatch should be displayed.
    impl fmt::Display for StopWatch {
        /// The default is to display the stop watch in milliseconds.
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            return write!(f, "{}ms", self.millis());
        }
    }

    #[allow(dead_code)]
    impl StopWatch {
        /// Returns a new instance of the stopwatch.
        pub fn new() -> StopWatch {
            StopWatch {
                start: None,
                duration: None,
            }
        }
        /// Returns a new instance of the stopwatch that has been started.
        pub fn start_new() -> StopWatch {
            StopWatch {
                start: Some(Instant::now()),
                duration: None,
            }
        }
        /// Starts or re-starts the stopwatch.
        pub fn start(&mut self) {
            self.start = Some(Instant::now());
            self.duration = None;
        }
        /// Stops the stopwatch.
        ///
        /// If the stop watch has not been started the duration will be set to 0 seconds.
        pub fn stop(&mut self) {
            match self.start {
                Some(start) => {
                    self.duration = Some(Instant::now() - start);
                    self.start = None
                }
                None => self.duration = Some(Duration::from_secs(0))
            }
        }
        /// Reset the stopwatch to it's initial values.
        pub fn reset(&mut self) {
            self.start = None;
            self.duration = None;
        }
        /// Returns the duration recorded in the stopwatch.
        pub fn elapsed(&self) -> Duration {
            if let Some(start) = self.start {
                Instant::now() - start
            } else if let Some(duration) = self.duration {
                duration
            } else {
                Duration::from_secs(0)
            }
        }
        /// Returns true if the stopwatch has been started.
        pub fn is_running(&self) -> bool {
            return self.start.is_some()
        }
        /// Returns how long the stop watch has been running.
        pub fn millis(&self) -> i64 {
            return self.elapsed().as_millis() as i64
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::thread;

        #[test]
        pub fn pristine() {
            let stop_watch = StopWatch::new();
            assert_eq!(stop_watch.is_running(), false);
            assert_eq!(stop_watch.elapsed().as_nanos(), 0);
        }

        #[test]
        pub fn start_new() {
            let stop_watch = StopWatch::start_new();
            assert!(stop_watch.is_running());
            thread::sleep(Duration::from_millis(10));
            assert!(stop_watch.elapsed().as_nanos() >= 10);
        }

        #[test]
        pub fn start_stop() {
            let mut stop_watch = StopWatch::new();
            assert!(!stop_watch.is_running());
            stop_watch.start();
            assert!(stop_watch.is_running());
            thread::sleep(Duration::from_millis(25));
            stop_watch.stop();
            assert!(!stop_watch.is_running());
            assert!(stop_watch.elapsed().as_millis() >= 25);
            // println!("{:?}", stop_watch);
        }

        #[test]
        pub fn reset() {
            let mut stop_watch = StopWatch::start_new();
            thread::sleep(Duration::from_millis(10));
            assert!(stop_watch.is_running());
            assert!(stop_watch.elapsed().as_millis() >= 10);
            stop_watch.reset();
            assert!(!stop_watch.is_running());
            assert_eq!(stop_watch.elapsed().as_millis(), 0);
        }
    }
}