//! Manages adding weather data history to the backing archive store.
use super::*;

pub use v3::{add_histories, store_size, loader};
mod v3 {
    //! The current version
    use super::*;

    /// Add weather histories to the archive.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather archive directory.
    /// * `daily_histories` is the collection of histories that will be added.
    pub fn add_histories<'dh>(
        weather_dir: &WeatherDir,
        daily_histories: &'dh DailyHistories,
    ) -> Result<Vec<&'dh History>> {
        let location = &daily_histories.location;
        let histories = &daily_histories.histories;
        let file = weather_dir.archive(&location.alias);
        let mut updater = WeatherHistoryUpdate::new(&location.alias, file)?;
        let dates_added = updater.add(histories)?;
        let mut histories_added = Vec::with_capacity(dates_added.len());
        histories.iter().for_each(|history| {
            if dates_added.iter().any(|date| &history.date == date) {
                histories_added.push(history)
            }
        });
        Ok(histories_added)
    }

    /// Get the size of a location archive.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `alias` is the location alias name.
    pub fn store_size(weather_dir: &WeatherDir, alias: &str) -> usize {
        weather_dir.archive(alias).size() as usize
    }

    pub mod loader {
        //! A threaded history data loader.
        use super::*;
        use std::{
            marker::PhantomData,
            sync::{mpsc, Arc, Mutex},
            thread,
        };
        use toolslib::{fmt::commafy, stopwatch::StopWatch};

        /// A helper to log elapsed load times.
        macro_rules! log_elapsed {
            ($what:expr, $count:expr, $stopwatch:expr) => {{
                let per_msec = $count as f64 / $stopwatch.millis() as f64;
                log::debug!(
                    "{:?} {}: {} loaded in {} ({:0.3}history/ms).",
                    thread::current().id(),
                    $what,
                    commafy($count),
                    $stopwatch,
                    per_msec
                );
            }};
        }

        /// The trait used by the [ArchiveLoader] to gather data from a weather archive.
        pub trait ArchiveProducer<T> {
            /// The *producer* side of the archive data.
            ///
            /// # Arguments
            ///
            /// * `sender` is used to hand off the gathered archive data.
            /// * `archives` is a collection of archives to gather data from.
            fn gather(&self, lid: i64, alias: &str, file: WeatherFile, sender: &mpsc::Sender<T>) -> Result<usize>;
            /// Trait boilerplate that gets archive metadata from the queue and calls the data extractor.
            fn send(&self, sender: mpsc::Sender<T>, archives: Arc<ArchiveQueue>) {
                while let Some(md) = archives.next() {
                    let mut load_time = StopWatch::start_new();
                    let filename = md.file.filename.clone();
                    match self.gather(md.lid, &md.alias, md.file, &sender) {
                        Ok(count) => {
                            load_time.stop();
                            self.log_elapsed(&md.alias, count, &load_time);
                        }
                        Err(err) => {
                            log::error!("{:?} error loading archive {} ({}).", thread::current().id(), filename, &err);
                            break;
                        }
                    }
                }
            }
            /// Trait boilerplate that logs elapsed time for the producer.
            ///
            /// # Arguments
            ///
            /// * `description` tersely describes the elapsed time.
            /// * `count` is the number of items mined from the archive.
            /// * `load_time` is how long the gather took.
            fn log_elapsed(&self, description: &str, count: usize, load_time: &StopWatch) {
                log_elapsed!(description, count, load_time);
            }
        }

        /// The trait used by the [ArchiveLoader] to collect the data gathered from weather archives.
        pub trait ArchiveConsumer<T> {
            /// The *consumer* side of the archive data.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the gathered archive data.
            fn collect(&mut self, receiver: mpsc::Receiver<T>) -> Result<usize>;
            /// The boilerplate side for the *consumer* of archive data.
            ///
            /// # Arguments
            ///
            /// * `receiver` is used to collect the gathered archive data.
            fn receive(&mut self, receiver: mpsc::Receiver<T>) {
                let mut load_time = StopWatch::start_new();
                match self.collect(receiver) {
                    Ok(count) => {
                        load_time.stop();
                        self.log_elapsed("Overall", count, &load_time);
                    }
                    Err(err) => {
                        let reason = format!("ArchiveConsumer collect error ({})", &err);
                        log::error!("{}", reason);
                    }
                }
            }
            /// Trait boilerplate that logs elapsed time for the consumer.
            ///
            /// # Arguments
            ///
            /// * `description` tersely describes the elapsed time.
            /// * `count` is the number of items mined from the archive.
            /// * `load_time` is how long the collection took.
            fn log_elapsed(&self, description: &str, count: usize, load_time: &StopWatch) {
                log_elapsed!(description, count, load_time);
            }
        }

        /// A threaded framework that gathers data from archives.
        #[derive(Debug)]
        pub struct ArchiveLoader<T> {
            /// The number of threads to use.
            threads: usize,
            /// The **`I need to be associated with a type`** compiler hack.
            phantom: PhantomData<T>,
        }
        impl<T: 'static + Send> ArchiveLoader<T> {
            /// Create a new instance of the loader.
            ///
            /// # Arguments
            ///
            /// * `threads` is the number of threads to use gathering data.
            pub fn new(threads: usize) -> ArchiveLoader<T> {
                Self { threads, phantom: PhantomData }
            }
            /// Gather data from a collection of archives.
            ///
            /// # Arguments
            ///
            /// * `archives` is the collection of archives data will be gathered from.
            /// * `producer` is used to create the threads that gather archive data.
            /// * `consumer` is used to create the collector of archive data.
            pub fn execute<P, C>(&mut self, archives: ArchiveQueue, producer: P, consumer: C) -> Result<()>
            where
                P: Fn() -> Box<dyn ArchiveProducer<T> + Send>,
                C: FnOnce() -> Box<dyn ArchiveConsumer<T> + Send>,
            {
                // start up the threads that gather data
                let archives = Arc::new(archives);
                let (sender, receiver) = mpsc::channel::<T>();
                let mut handles = Vec::with_capacity(self.threads);
                for _ in 0..self.threads {
                    let producer = producer();
                    let sender = sender.clone();
                    let archive_queue = archives.clone();
                    let handle = thread::spawn(move || {
                        producer.send(sender, archive_queue);
                    });
                    handles.push(handle);
                }
                // now that the threads are running close down the sender
                drop(sender);
                // run the consumer
                consumer().receive(receiver);
                // now cleanup the threads
                for handle in handles {
                    let thread_id = handle.thread().id();
                    match handle.join() {
                        Ok(_) => (),
                        Err(_) => {
                            log::error!("Error joining with thread ({:?})", thread_id);
                        }
                    }
                }
                Ok(())
            }
        }

        /// The archive metadata used by the [ArchiveQueue].
        #[derive(Debug)]
        pub struct ArchiveQueueMd {
            /// The database primary id of the weather location.
            pub lid: i64,
            /// The weather location alias name.
            pub alias: String,
            /// The weather data archive.
            pub file: WeatherFile,
        }

        /// A thread-safe collection of weather archive metadata used by the [ArchiveLoader].
        #[derive(Debug)]
        pub struct ArchiveQueue(Mutex<Vec<ArchiveQueueMd>>);
        impl ArchiveQueue {
            pub fn new(conn: &Connection, weather_dir: &WeatherDir) -> Result<Self> {
                let id_alias_files: Vec<ArchiveQueueMd> = locations::id_aliases(conn)?
                    .into_iter()
                    .map(|(lid, alias)| {
                        let file = weather_dir.archive(&alias);
                        ArchiveQueueMd { lid, alias, file }
                    })
                    .collect();
                Ok(Self(Mutex::new(id_alias_files)))
            }
            pub fn next(&self) -> Option<ArchiveQueueMd> {
                match self.0.lock() {
                    Ok(mut guard) => guard.pop(),
                    Err(err) => err.into_inner().pop(),
                }
            }
        }
    }
}
