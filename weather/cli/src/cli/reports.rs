//! The Weather Data reports.
pub mod list_history;
pub mod list_locations;
pub mod list_summary;
pub mod report_history;

// link to csv under another name to prevent confusion with the internal csv modules
extern crate csv as csv_lib;
use serde_json::{json, map::Map, Value};
use toolslib::{header, layout, report::ReportSheet, text};

macro_rules! csv_write_record {
    ($writer:expr, $row:expr) => {
        if let Err(err) = $writer.write_record($row) {
            log::error!("Failed to write CSV record ({}).", err)
        }
    };
}
/// Attempts to write a `CSV` record and captures any errors that may occur.
///
use csv_write_record;

/// Convert a `JSON` document into a string.
///
/// # Arguments
///
/// - `json` is the document that will be converted into a string.
/// - `pretty` controls if the document will be pretty printed or not.
///
fn json_to_string(json: Value, pretty: bool) -> String {
    let result = match pretty {
        true => serde_json::to_string_pretty(&json),
        false => serde_json::to_string(&json),
    };
    result.unwrap_or_else(|err| {
        // to_string should always succeed... famous last words...
        log::error!("Failed to write JSON ({}).", err);
        String::default()
    })
}

/// Convert a `CSV` document into a string.
///
/// # Arguments
///
/// - `writer` is the document that will be converting into a string.
///
fn csv_to_string(writer: csv::Writer<Vec<u8>>) -> String {
    match writer.into_inner() {
        Ok(content) => String::from_utf8(content).unwrap_or_else(|err| {
            log::error!("Did not convert CSV to string ({}).", err);
            Default::default()
        }),
        Err(err) => {
            log::error!("Did not get content from CSV writer ({}).", err);
            Default::default()
        }
    }
}

macro_rules! text_title_separator {
    ($columns:expr) => {
        (0..$columns).into_iter().map(|_| text!(+ "-")).collect()
    };
}
/// Create separators between header rows and text rows.
///
use text_title_separator;
