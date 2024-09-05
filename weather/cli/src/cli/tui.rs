//! The Terminal based weather UI.

use super::*;
use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Position, Size},
    prelude::*,
};
use std::{
    cmp,
    fmt::{Debug, Formatter},
    ops::ControlFlow,
    rc::Rc,
};
use termui_lib::prelude::*;
use weather_lib::prelude::{DataCriteria, Location, LocationCriteria, WeatherData};

pub use app::run as weather_ui;
mod app;

mod dialogs;
mod histories_win;
mod locations_win;
mod summary_win;

/// Get an iterator of lowercase and uppercase ASCII characters.
///
fn alpha() -> impl Iterator<Item = char> {
    ('a'..='z').chain('A'..='Z')
}

/// Get an iterator of numeric digits.
///
fn digits() -> impl Iterator<Item = char> {
    '0'..='9'
}

/// Get an iterator of lowercase and uppercase ASCII characters including numeric digits.
///
fn alphanumeric() -> impl Iterator<Item = char> {
    alpha().chain(digits())
}

/// Validate a date string represents a valid date.
///
fn validate_date(name: &str, date_str: &str) -> std::result::Result<NaiveDate, String> {
    // let date = field.text();
    match date_str.chars().any(|ch| ch.is_whitespace()) {
        true => Err(format!("{} date contains whitespace.", name)),
        false => match toolslib::date_time::parse_date(date_str) {
            Err(_) => Err(format!("{} date ({}) is not valid", name, date_str)),
            Ok(date) => Ok(date),
        },
    }
}
