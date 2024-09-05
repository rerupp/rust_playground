//! The data model for weather data locations.
use super::*;

pub use v2::{create as weather_locations, search as search_locations};
mod v2 {
    //! The data model for weather data locations.
    //!
    use super::*;
    // use crate::backend::db::uscities;
    use crate::backend::db::us_cities as uscities;
    use serde_json as json;
    use std::io::{BufWriter, Write};

    /// The name of the locations document in the weather data directory.
    pub const LOCATIONS_FILENAME: &'static str = "locations.json";

    /// The name of the updated locations document in the weather data directory.
    pub const LOCATIONS_UPDATE_FILE: &'static str = "locations.upd";

    /// The name of the backup locations document in the weather data directory.
    pub const LOCATIONS_BACKUP_FILE: &'static str = "locations.bck";

    /// Search the US cities database for locations.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `criteria` is used to filter the search results.
    ///
    pub fn search(config: &Config, criteria: LocationCriteria) -> Result<Vec<Location>> {
        let us_cities = uscities::UsCities::try_from(config)?;
        match us_cities.search(criteria) {
            Ok(locations) => Ok(locations),
            Err(err) => Err(Error::from(err.to_string())),
        }
    }

    /// The [LocationsDocument] error builder.
    macro_rules! locations_err {
        ($reason:expr) => {
            Error::from(format!("Locations: {}", $reason))
        };
    }

    /// Logs the location error and return it as a [Result].
    macro_rules! error {
        ($reason:expr) => {{
            let error = locations_err!($reason);
            log::error!("{}", error);
            Err(error)
        }};
    }

    /// Creates an instance of the [LocationsDocument] collection.
    pub fn create(weather_dir: &WeatherDir) -> Result<LocationsDocument> {
        let file = weather_dir.file(LOCATIONS_FILENAME);
        if file.exists() {
            LocationsDocument::new(file)
        } else {
            log::warn!("{} does not exist", file);
            Ok(LocationsDocument(vec![]))
        }
    }

    /// The locations `JSON` document manager.
    #[derive(Debug)]
    pub struct LocationsDocument(
        /// The location metadata.
        Vec<LocationMd>,
    );
    /// Create a new instance of the manager.
    impl LocationsDocument {
        /// Get a new instance of the location metadata.
        ///
        /// # Arguments
        ///
        /// * `file` contains the weather data locations.
        pub fn new(mut file: WeatherFile) -> Result<Self> {
            file.refresh();
            if file.exists() {
                let reader = file.reader()?;
                let result: std::result::Result<LocationsMd, json::Error> = json::from_reader(reader);
                match result {
                    Ok(md) => Ok(Self(md.locations)),
                    Err(err) => {
                        let reason = format!("Error loading JSON from {}: {}", &file, &err);
                        Err(locations_err!(reason))
                    }
                }
            } else {
                log::warn!("Locations file does not exist...");
                Ok(Self(Vec::with_capacity(0)))
            }
        }
        /// Get the number of locations that have been loaded.
        pub fn cnt(&self) -> usize {
            self.0.len()
        }
        /// Creates an iterator returning weather data locations.
        ///
        /// # Arguments
        ///
        /// * `filters` are used to scope which locations will be returned.
        /// * `icase` will make filters case-sensitive (`true`) or ignore case (`false`).
        /// * `sort` will order the matching locations by their name.
        pub fn as_iter(&self, patterns: &Vec<String>, icase: bool, sort: bool) -> LocationsIter {
            let mut locations: Vec<&LocationMd> = if patterns.is_empty() {
                self.0.iter().collect()
            } else {
                let prepare = |text: &str| if icase { text.to_string() } else { text.to_lowercase() };
                let patterns: Vec<String> = patterns.iter().map(|pattern| prepare(pattern)).collect();
                self.0
                    .iter()
                    .filter(|location| {
                        let name = prepare(&location.name);
                        let alias = prepare(&location.alias);
                        patterns.iter().any(|pattern| is_match(&name, &alias, pattern))
                    })
                    .collect()
            };
            if sort {
                locations.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
            }
            LocationsIter::new(locations)
        }
        /// Add a location to the locations document.
        ///
        /// # Arguments
        ///
        /// * `location` is the location that will be added.
        /// * `weather_dir` is the weather data directory.
        pub fn add(&mut self, mut location: Location, weather_dir: &WeatherDir) -> Result<()> {
            validate(&location)?;
            // make sure the alias is lowercase
            location.alias = location.alias.to_lowercase();
            match self.0.iter().find(|md| md.alias == location.alias) {
                Some(location) => {
                    error!(format!("{} already uses the '{}' alias name", location.name, location.alias))
                }
                None => {
                    let alias = location.alias.clone();
                    self.0.push(LocationMd::from(location));
                    save_locations(weather_dir, &self.0)?;
                    let archive = weather_dir.archive(&alias);
                    WeatherArchive::create(&alias, archive)?;
                    Ok(())
                }
            }
        }
    }

    /// Do a high level validation of the new location.
    ///
    /// # Arguments
    ///
    /// * `location` is what will be validated.
    fn validate(location: &Location) -> Result<()> {
        if location.name.is_empty() {
            error!("The location name cannot be empty.")
        } else if location.alias.is_empty() {
            error!("The alias name cannot be empty.")
        } else if location.longitude.is_empty() {
            error!("Longitude cannot be empty.")
        } else if location.longitude.parse::<f64>().is_err() {
            error!("Longitude is not valid.")
        } else if location.latitude.is_empty() {
            error!("Latitude cannot be empty.")
        } else if location.latitude.parse::<f64>().is_err() {
            error!("Latitude is not valid.")
        } else if location.tz.is_empty() {
            error!("Timezone cannot be empty.")
        } else {
            for tz in chrono_tz::TZ_VARIANTS.iter() {
                let tz_name = tz.name();
                if location.tz == tz_name {
                    return Ok(());
                } else if location.tz.to_lowercase() == tz_name.to_lowercase() {
                    return error!(format!("Timezone is invalid. Did you mean {}?", tz_name));
                }
            }
            error!(format!("Timezone {} is not valid.", location.tz))
        }
    }

    /// Save the locations collection to the weather data document.
    ///
    /// # Arguments
    ///
    /// * `weather_dir` is the weather data directory.
    /// * `locations` is the locations metadata.
    fn save_locations(weather_dir: &WeatherDir, locations: &Vec<LocationMd>) -> Result<()> {
        let mut update_file = weather_dir.file(LOCATIONS_UPDATE_FILE);
        if update_file.exists() {
            update_file.remove()?;
        }
        update_file.touch()?;
        let mut writer = BufWriter::new(update_file.writer()?);
        let locations_md = LocationsMd { locations: locations.iter().map(|location| location.clone()).collect() };
        match serde_json::to_writer_pretty(&mut writer, &locations_md) {
            Ok(_) => match writer.flush() {
                Ok(_) => {
                    drop(writer);
                    let mut original_file = weather_dir.file(LOCATIONS_FILENAME);
                    let mut backup_file = weather_dir.file(LOCATIONS_BACKUP_FILE);
                    if original_file.exists() {
                        original_file.rename(&mut backup_file)?;
                    }
                    update_file.rename(&mut original_file)?;
                    backup_file.remove()
                }
                Err(err) => error!(format!("Error flushing {} ({}).", update_file.filename, err)),
            },
            Err(err) => error!(format!("Error saving {} ({}).", update_file.filename, err)),
        }
    }

    /// Test if location name or alias matches a pattern.
    ///
    /// The pattern can include an `*` to denote a match of any leading characters, any trailing characters, or
    /// if the pattern should be considered a sub-string match.
    ///
    /// # Arguments
    ///
    /// * `name` is the location name that will be matched against the pattern.
    /// * `alias` is the location alias that will be matched against the pattern.
    /// * `pattern` is what will be matched against the location name and alias.
    fn is_match(name: &String, alias: &String, pattern: &String) -> bool {
        if pattern == "*" {
            true
        } else if pattern.starts_with("*") && pattern.ends_with("*") {
            let slice = &pattern[1..pattern.len() - 1];
            name.contains(slice) || alias.contains(slice)
        } else if pattern.starts_with("*") {
            let slice = &pattern[1..];
            name.ends_with(slice) || alias.ends_with(slice)
        } else if pattern.ends_with("*") {
            let slice = &pattern[..pattern.len() - 1];
            name.starts_with(slice) || alias.starts_with(slice)
        } else {
            name == pattern || alias == pattern
        }
    }

    use serde::{Deserialize, Serialize};

    /// The bean that describes the locations `JSON` document.
    #[derive(Debug, Deserialize, Serialize)]
    struct LocationsMd {
        /// The collection of location metadata.
        locations: Vec<LocationMd>,
    }

    /// The bean that describes the metadata for a location.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct LocationMd {
        /// The name of a location.
        name: String,
        /// A unique nickname of a location.
        alias: String,
        /// The location longitude.
        longitude: String,
        /// The location latitude.
        latitude: String,
        /// the location timezone.
        tz: String,
    }
    impl From<&LocationMd> for Location {
        /// Convert the `JSON` location metadata to a [Location].
        fn from(md: &LocationMd) -> Self {
            Self {
                name: md.name.clone(),
                alias: md.alias.clone(),
                longitude: md.longitude.clone(),
                latitude: md.latitude.clone(),
                tz: md.tz.clone(),
            }
        }
    }
    impl From<Location> for LocationMd {
        fn from(location: Location) -> Self {
            Self {
                name: location.name,
                alias: location.alias,
                longitude: location.longitude,
                latitude: location.latitude,
                tz: location.tz,
            }
        }
    }

    /// An iterator over the `JSON` location metadata.
    #[derive(Debug)]
    pub struct LocationsIter<'l> {
        /// The collection of `JSON` location metadata.
        location_md: Vec<&'l LocationMd>,
        /// The next location metadata that will be returned.
        index: usize,
    }
    impl<'l> LocationsIter<'l> {
        fn new(location_md: Vec<&'l LocationMd>) -> Self {
            Self { location_md, index: 0 }
        }
    }
    impl<'l> Iterator for LocationsIter<'l> {
        type Item = Location;
        /// Get the next `JSON` location metadata as a [Location].
        fn next(&mut self) -> Option<Self::Item> {
            match self.index < self.location_md.len() {
                true => {
                    let location_md = self.location_md[self.index];
                    self.index += 1;
                    Some(Location::from(location_md))
                }
                false => None,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::backend::testlib;

        #[test]
        fn matches() {
            let target = "start".to_string();
            let testcase = "st*".to_string();
            assert!(is_match(&target, &String::default(), &testcase));
            assert!(is_match(&String::default(), &target, &testcase));
            assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
            let target = "end".to_string();
            let testcase = "*d".to_string();
            assert!(is_match(&target, &String::default(), &testcase));
            assert!(is_match(&String::default(), &target, &testcase));
            assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
            let target = "middle".to_string();
            let testcase = "*dd*".to_string();
            assert!(is_match(&target, &String::default(), &testcase));
            assert!(is_match(&String::default(), &target, &testcase));
            assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
            let target = "exact".to_string();
            let testcase = "exact".to_string();
            assert!(is_match(&target, &String::default(), &testcase));
            assert!(is_match(&String::default(), &target, &testcase));
            assert_eq!(is_match(&String::default(), &String::default(), &testcase), false);
        }

        #[test]
        fn as_iter() {
            let resources = testlib::test_resources().join("filesys");
            let weather_dir = WeatherDir::new(resources).unwrap();
            let testcase = LocationsDocument::new(weather_dir.file("locations.json")).unwrap();
            assert_eq!(testcase.0.len(), 3);
            // no filters
            let mut result = testcase.as_iter(&vec![], false, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Northern City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
            // starts with
            let patterns = vec!["Bet*".to_string(), "nOrth*".to_string(), "South*".to_string()];
            let mut result = testcase.as_iter(&patterns, true, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
            let mut result = testcase.as_iter(&patterns, false, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Northern City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
            // ends with
            let patterns = vec!["*en City".to_string(), "*RN city".to_string()];
            let mut result = testcase.as_iter(&patterns, true, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert!(result.next().is_none());
            let mut result = testcase.as_iter(&patterns, false, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Northern City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
            // contains
            let patterns = vec!["*et*".to_string(), "*OUT*".to_string()];
            let mut result = testcase.as_iter(&patterns, true, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert!(result.next().is_none());
            let mut result = testcase.as_iter(&patterns, false, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
            // exact
            let patterns = vec!["north".to_string(), "South".to_string(), "between".to_string()];
            let mut result = testcase.as_iter(&patterns, true, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Northern City");
            assert!(result.next().is_none());
            let mut result = testcase.as_iter(&patterns, false, true);
            assert_eq!(&result.next().unwrap().name, "Between City");
            assert_eq!(&result.next().unwrap().name, "Northern City");
            assert_eq!(&result.next().unwrap().name, "Southern City");
            assert!(result.next().is_none());
        }

        #[test]
        fn from() {
            let md = LocationMd {
                name: "Name".to_string(),
                alias: "Alias".to_string(),
                longitude: "1.23".to_string(),
                latitude: "-10.3".to_string(),
                tz: "UTC".to_string(),
            };
            let testcase = Location::from(&md);
            assert_eq!(testcase.name, md.name);
            assert_eq!(testcase.alias, md.alias);
            assert_eq!(testcase.longitude, md.longitude);
            assert_eq!(testcase.latitude, md.latitude);
            assert_eq!(testcase.tz, md.tz);
        }
    }
}