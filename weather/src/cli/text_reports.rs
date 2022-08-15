//! # A text based report generator
//!
//! The intent of this module is to provide a common text based reporting engine.
//! There was so much commonality between the various text based reporting commands it
//! seemed reasonable to build a common reporting engine.
//!
//! The components allow text to be placed into report columns, abstracting how the text is
//! really generated. At some point I would think defining a set of macros to help genearate
//! the output will be in order.
use std::io::{self, Write};

use chrono::prelude::*;

use crate::cli::{CliError, CliResult, ReportWriter};

/// Indicate what alignment a text column will have.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Alignment {
    /// Text will be aligned on the left hand side of a column.
    Left,
    /// Text will be centered in a column.
    Center,
    /// Text will be aligned on the right hand side of a column.
    Right,
}

/// The properties of a text column.
///
/// The properties act as defaults for report generation. They can be overridden as the
/// report is being built.
pub struct ColumnProperty {
    /// The maximum width of text added to the column.
    pub width: usize,
    /// The default alignment of text in a column.
    pub default_alignment: Alignment,
}

impl ColumnProperty {
    /// Creates an instance of the default column properties.
    ///
    /// # Arguments
    ///
    /// * `default_alignment` - the default alignment for the text column.
    pub fn new(default_alignment: Alignment) -> ColumnProperty {
        ColumnProperty {
            width: 0,
            default_alignment,
        }
    }
    /// Sets the minimum width of a column.
    ///
    /// # Arguments
    ///
    /// * `minimum_width` - the minimum number of characters in the column.
    pub fn with_minimum_width(mut self, minimum_width: usize) -> Self {
        self.width = minimum_width;
        self
    }
}

/// A container for text in a column.
///
/// The column text is not formatted until the report is generated. Leading
/// and trailing white space *will NOT* be trimmed during report generation.
#[derive(Debug)]
pub struct ColumnContent {
    /// The column text.
    pub content: String,
    /// The text alignment.
    ///
    /// If the alignment in *None*, alignment will come from the column property default.
    pub alignment: Option<Alignment>,
}

impl ColumnContent {
    /// Creates a new instance of the column content.
    ///
    /// # Arguments
    ///
    /// * `text` - the report column text.
    pub fn new(text: &str) -> ColumnContent {
        ColumnContent {
            content: text.to_string(),
            alignment: None,
        }
    }
    /// Set the alignment of text in a column.
    ///
    /// # Arguments
    ///
    /// * `alignment` - The alignment of column text.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

/// A container for the report column properties.
pub struct ReportColumns(Vec<ColumnProperty>);

impl ReportColumns {
    /// Creates the container of report column properties.
    ///
    /// # Arguments
    ///
    /// * `columns` - the initial set of column properties being defined.
    pub fn new(columns: Vec<ColumnProperty>) -> ReportColumns {
        ReportColumns(columns)
    }
    /// Add column properties to the container.
    ///
    /// Returns a reference to the container to allow method chaining.
    ///
    /// # Arguments
    ///
    /// * `columns` - the column properties to add.
    pub fn add_columns(&mut self, columns: Vec<ColumnProperty>) -> &mut Self {
        for column in columns {
            self.0.push(column)
        }
        self
    }
    /// Get a column property by index in the container.
    ///
    /// Return a reference to the column property or `None` if the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index` - the index of the column property to return.
    pub fn get(&self, index: usize) -> Option<&ColumnProperty> {
        self.0.get(index)
    }
    /// Returns the number of properties in the container.
    pub fn column_count(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod column_tests {
    use super::*;

    #[test]
    fn column_property() {
        let testcase = ColumnProperty::new(Alignment::Left);
        assert_eq!(testcase.width, 0);
        assert_eq!(testcase.default_alignment, Alignment::Left);
        let testcase = ColumnProperty::new(Alignment::Center)
            .with_minimum_width(10);
        assert_eq!(testcase.width, 10);
        assert_eq!(testcase.default_alignment, Alignment::Center);
    }

    #[test]
    fn column_content() {
        let testcase = ColumnContent::new("foobar");
        assert_eq!(testcase.content, "foobar".to_string());
        assert_eq!(testcase.alignment, None);
        let testcase = ColumnContent::new("text").with_alignment(Alignment::Left);
        assert_eq!(testcase.content, "text".to_string());
        assert_eq!(testcase.alignment, Some(Alignment::Left));
    }

    #[test]
    fn report_columns() {
        assert_eq!(ReportColumns::new(vec![]).column_count(), 0);
        let mut testcase = ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Left),
        ]);
        assert_eq!(testcase.column_count(), 1);
        testcase.add_columns(vec![
            ColumnProperty::new(Alignment::Center),
            ColumnProperty::new(Alignment::Right).with_minimum_width(10),
        ]);
        assert_eq!(testcase.column_count(), 3);
        assert_eq!(testcase.0.get(0).unwrap().default_alignment, Alignment::Left);
        assert_eq!(testcase.0.get(0).unwrap().width, 0);
        assert_eq!(testcase.0.get(1).unwrap().default_alignment, Alignment::Center);
        assert_eq!(testcase.0.get(1).unwrap().width, 0);
        assert_eq!(testcase.0.get(2).unwrap().default_alignment, Alignment::Right);
        assert_eq!(testcase.0.get(2).unwrap().width, 10);
    }
}

/// Identifies the type of a line in the report.
#[derive(Debug, PartialEq)]
pub enum RecordType {
    /// The report line is a header.
    Header,
    /// The report line contains details.
    Detail,
    /// The report line columns will consist of the separator character.
    Separator(char),
}

/// Constructs a line that can be added to a report.
pub struct RecordBuilder {
    /// The type of line that is being built.
    record_type: RecordType,
    /// The column contents of the line.
    pub columns_content: Vec<ColumnContent>,
}

impl RecordBuilder {
    /// Creates a new instance of the builder.
    ///
    /// The report generator uses this to create an appropriate report
    /// entry. It is not intended to be used outside the text report module.
    ///
    /// # Arguments
    ///
    /// * `record_type` - the type of record that will be constructed.
    fn new(record_type: RecordType) -> RecordBuilder {
        RecordBuilder { record_type, columns_content: vec![] }
    }
    /// Add a column to the report line.
    ///
    /// The function returns a mutable reference to the container to allow method chaining.
    ///
    /// # Arguments
    ///
    /// * `column_content` - the column text.
    ///
    pub fn add_content(&mut self, column_content: ColumnContent) -> &mut Self {
        self.columns_content.push(column_content);
        self
    }
    /// Add a collection of columns to the report line.
    ///
    /// The function returns a mutable reference to the container to allow method chaining.
    ///
    /// # Arguments
    ///
    /// * `column_contents` - The collection of column text that will be added.
    ///
    pub fn add_contents(&mut self, columns_content: Vec<ColumnContent>) -> &mut Self {
        for column_content in columns_content {
            self.columns_content.push(column_content);
        }
        self
    }
    /// Add a collection of columns to the report line.
    ///
    /// The function returns the container to allow method chaining. This will typically
    /// be used to allow a new instance of the container to have initial content.
    ///
    /// # Arguments
    ///
    /// * `columns_content` - The collection of column text that will be added.
    ///
    pub fn with_contents(mut self, columns_content: Vec<ColumnContent>) -> Self {
        self.add_contents(columns_content);
        self
    }
    /// Returns the number of text columns contained in the container.
    ///
    pub fn column_count(&self) -> usize {
        self.columns_content.len()
    }
    /// Formats columns in container for report generation.
    ///
    /// The contents of the formatted columns depends on the record type.
    ///
    /// * Separator line columns will consist of separator characters filled to the column width.
    /// * Header and detail line columns will be filled with white space to the column width. The
    /// returned column content will be empty if the line column count does not match the report
    /// column count.
    ///
    /// # Arguments
    ///
    /// * `report_columns` - the report column properties.
    ///
    fn format_contents(&self, report_columns: &ReportColumns) -> Vec<String> {
        let mut contents: Vec<String> = vec![];
        if let RecordType::Separator(separator) = self.record_type {
            let separator = separator.to_string();
            for column_property in &report_columns.0 {
                contents.push(separator.repeat(column_property.width));
            }
        } else {
            let column_count = report_columns.column_count();
            if column_count == self.columns_content.len() {
                for i in 0..column_count {
                    let column_content = &self.columns_content[i];
                    let column_property = &report_columns.0[i];
                    let column_alignment = match column_content.alignment {
                        Some(alignment) => alignment,
                        _ => column_property.default_alignment,
                    };
                    let column_content = align_text(
                        &column_content.content,
                        column_property.width,
                        column_alignment,
                    );
                    contents.push(column_content);
                }
            }
        }
        contents
    }
}

#[cfg(windows)]
/// The new line character string. Thanks MS-DOS for this silliness...
pub const NL: &'static str = "\r\n";
#[cfg(not(windows))]
/// The new line character string for non-Windows platforms.
pub const NL: &'static str = "\n";

/// The public facing builder to create text reports.
pub struct ReportBuilder {
    /// The column properties for the report.
    pub report_columns: ReportColumns,
    /// The lines of the report.
    pub content: Vec<RecordBuilder>,
}

impl ReportBuilder {
    /// Creates a new instance of the report builder.
    ///
    /// # Arguments
    ///
    /// * `report_columns` - the report column properties.
    pub fn new(report_columns: ReportColumns) -> ReportBuilder {
        ReportBuilder {
            report_columns,
            content: vec![],
        }
    }
    /// Returns a new instance of the record builder with the record type set to header.
    ///
    pub fn header() -> RecordBuilder {
        RecordBuilder::new(RecordType::Header)
    }
    /// Returns a new instance of the record builder with the record type set to detail.
    pub fn detail() -> RecordBuilder {
        RecordBuilder::new(RecordType::Detail)
    }
    /// Adds a separator line to the report builder.
    ///
    /// # Arguments
    ///
    /// * `separator` - the character that will be used to create the separator line.
    pub fn add_separator(&mut self, separator: char) {
        let record_builder = RecordBuilder::new(RecordType::Separator(separator));
        self.add_contents(record_builder).unwrap();
    }
    /// Add a record to the report builder.
    ///
    /// If the record is a header or detail line, an error will be returned unless the
    /// record column count matches the report column count.
    ///
    /// # Arguments
    ///
    /// * `record_builder` - The record that will be added to the report.
    ///
    pub fn add_contents(&mut self, record_builder: RecordBuilder) -> CliResult<()> {
        match record_builder.record_type {
            RecordType::Separator(_) => {
                self.content.push(record_builder);
                Ok(())
            }
            _ => {
                if record_builder.column_count() != self.report_columns.column_count() {
                    let msg = format!(
                        "Record column count {} does not match report column count {}!",
                        record_builder.column_count(), self.report_columns.column_count()
                    );
                    Err(CliError::new(&msg))
                } else {
                    for i in 0..record_builder.column_count() {
                        self.report_columns.0[i].width = std::cmp::max(
                            self.report_columns.0[i].width,
                            record_builder.columns_content[i].content.len(),
                        );
                    }
                    self.content.push(record_builder);
                    Ok(())
                }
            }
        }
    }
    /// Creates and writes the report.
    ///
    /// An error will be returned if there are problems writing the report.
    ///
    /// # Arguments
    ///
    /// * `report_writer` - used to create the destination of the report.
    ///
    pub fn output(&self, report_writer: &ReportWriter) -> CliResult<()> {
        let mut writer = report_writer.create()?;
        let mut write_output = || -> io::Result<()> {
            for record in self {
                writer.write_all(record.as_bytes())?;
                writer.write_all(NL.as_bytes())?;
            }
            Ok(())
        };
        match write_output() {
            Err(error) => Err(CliError::new(&error.to_string())),
            _ => Ok(())
        }
    }
}

/// Allows the report builder to be converted to an iterator that returns a reference to the
/// report contents.
impl<'a> IntoIterator for &'a ReportBuilder {
    type Item = String;
    /// The iterator implementation for a report builder instance.
    type IntoIter = ReportIterator<'a>;
    /// Creates the report builder iterator.
    fn into_iter(self) -> Self::IntoIter {
        ReportIterator {
            report_builder: self,
            content_index: 0,
        }
    }
}

/// The report builder iterator.
pub struct ReportIterator<'a> {
    /// A reference to the report builder instance.
    report_builder: &'a ReportBuilder,
    /// The report line returned when `next` is called.
    content_index: usize,
}

/// The report builder iterator responsible for creating the records of a report.
impl<'a> Iterator for ReportIterator<'a> {
    type Item = String;
    /// For each report record, columns are formatted and then joined to create the line of text
    /// written to report.
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.content_index;
        if index >= self.report_builder.content.len() {
            None
        } else {
            self.content_index += 1;
            let record_builder: &RecordBuilder = &self.report_builder.content[index];
            let contents = record_builder.format_contents(&self.report_builder.report_columns);
            if contents.is_empty() {
                None
            } else {
                Some(contents.join(" "))
            }
        }
    }
}

#[cfg(test)]
mod report_tests {
    use super::*;

    #[test]
    fn record_builder() {
        let testcase = RecordBuilder::new(RecordType::Header);
        assert_eq!(testcase.record_type, RecordType::Header);
        assert_eq!(testcase.columns_content.len(), 0);
        let testcase = RecordBuilder::new(RecordType::Detail);
        assert_eq!(testcase.record_type, RecordType::Detail);
        assert_eq!(testcase.columns_content.len(), 0);
        let mut testcase = RecordBuilder::new(RecordType::Detail)
            .with_contents(vec![
                ColumnContent::new("foo")
            ]);
        assert_eq!(testcase.columns_content.len(), 1);
        testcase.add_contents(vec![
            ColumnContent::new("bar"),
            ColumnContent::new("foobar"),
        ]);
        assert_eq!(testcase.columns_content.len(), 3);
        assert_eq!(&testcase.columns_content.get(0).unwrap().content, "foo");
        assert_eq!(&testcase.columns_content.get(1).unwrap().content, "bar");
        assert_eq!(&testcase.columns_content.get(2).unwrap().content, "foobar");
    }

    #[test]
    fn record_builder_format_contents() {
        let report_columns = ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Right).with_minimum_width(5),
            ColumnProperty::new(Alignment::Center).with_minimum_width(5),
            ColumnProperty::new(Alignment::Left).with_minimum_width(5),
        ]);
        let testcase = RecordBuilder {
            record_type: RecordType::Detail,
            columns_content: vec![
                ColumnContent::new("foo").with_alignment(Alignment::Left),
                ColumnContent::new("foo"),
                ColumnContent::new("foo").with_alignment(Alignment::Right),
            ],
        };
        assert_eq!(testcase.format_contents(&report_columns), ["foo  ", " foo ", "  foo"]);
        let testcase = RecordBuilder { record_type: RecordType::Separator('='), columns_content: vec![] };
        assert_eq!(testcase.format_contents(&report_columns), ["=====", "=====", "====="]);
        let testcase = RecordBuilder {
            record_type: RecordType::Header,
            columns_content: vec![
                ColumnContent::new("foo").with_alignment(Alignment::Left),
                ColumnContent::new("foo"),
            ],
        };
        assert!(testcase.format_contents(&report_columns).is_empty());
    }

    #[test]
    fn report_header_builder() {
        let testcase = ReportBuilder::header();
        assert_eq!(testcase.record_type, RecordType::Header);
        assert_eq!(testcase.columns_content.len(), 0);
    }

    #[test]
    fn report_detail_builder() {
        let testcase = ReportBuilder::detail();
        assert_eq!(testcase.record_type, RecordType::Detail);
        assert_eq!(testcase.columns_content.len(), 0);
    }

    #[test]
    fn report_add_content() {
        let mut testcase = ReportBuilder::new(ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Left),
            ColumnProperty::new(Alignment::Center).with_minimum_width(20),
        ]));
        let headers = ReportBuilder::header().with_contents(vec![
            ColumnContent::new("first"),
            ColumnContent::new("second"),
        ]);
        assert!(testcase.add_contents(headers).is_ok());
        assert_eq!(testcase.report_columns.0[0].width, "first".len());
        assert_eq!(testcase.report_columns.0[1].width, 20);
        let details = ReportBuilder::detail().with_contents(vec![
            ColumnContent::new("first column"),
            ColumnContent::new("second column"),
        ]);
        assert!(testcase.add_contents(details).is_ok());
        assert_eq!(testcase.report_columns.0[0].width, "first column".len());
        assert_eq!(testcase.report_columns.0[1].width, 20);
        assert!(testcase.add_contents(ReportBuilder::detail()).is_err());
        let too_many_details = ReportBuilder::detail().with_contents(vec![
            ColumnContent::new("first column"),
            ColumnContent::new("second column"),
            ColumnContent::new("third column"),
        ]);
        assert!(testcase.add_contents(too_many_details).is_err());
    }

    #[test]
    fn report_builder_iterator() {
        let mut testcase = ReportBuilder::new(ReportColumns::new(vec![
            ColumnProperty::new(Alignment::Left),
            ColumnProperty::new(Alignment::Center),
            ColumnProperty::new(Alignment::Right),
        ]));
        assert!(testcase.add_contents(ReportBuilder::header().with_contents(vec![
            ColumnContent::new("Header1"),
            ColumnContent::new("Header2"),
            ColumnContent::new("Header3"),
        ])).is_ok());
        testcase.add_separator('-');
        assert!(testcase.add_contents(ReportBuilder::detail().with_contents(vec![
            ColumnContent::new("data1"),
            ColumnContent::new("data2"),
            ColumnContent::new("data3"),
        ])).is_ok());
        let mut iter = testcase.into_iter();
        assert_eq!(iter.next(), Some("Header1 Header2 Header3".to_string()));
    }

    #[test]
    fn align_text_util() {
        assert_eq!(align_text("foo", 5, Alignment::Left), "foo  ");
        assert_eq!(align_text("foo", 5, Alignment::Center), " foo ");
        assert_eq!(align_text("foo", 5, Alignment::Right), "  foo");
    }

    #[test]
    fn fmt_float_util() {
        assert_eq!(fmt_float(&None, 5), "");
        assert_eq!(fmt_float(&Some(1.236), 4), "1.2360");
        assert_eq!(fmt_float(&Some(1.236), 2), "1.24");
    }
}

/// Creates an string aligned to a specific width.
///
/// The string *will NOT* be truncated if the width is less than the string length.
///
/// # Arguments
///
/// * `value` - the text that will be formatted.
/// * `width` - the minimum width of the formatted text.
/// * `alignment` - the alignment of the text.
///
pub fn align_text(value: &str, width: usize, alignment: Alignment) -> String {
    match alignment {
        Alignment::Left => format!("{:1$}", value, width),
        Alignment::Right => format!("{:>1$}", value, width),
        Alignment::Center => format!("{:^1$}", value, width),
    }
}

/// Creates a string representation of a float value.
///
/// If the float value is `None` an empty string will be returned.
///
/// # Arguments
///
/// * `value` - the optional float value.
/// * `precision` - the precision of the float being converted.
///
pub fn fmt_float(value: &Option<f64>, precision: usize) -> String {
    if let Some(float) = value {
        format!("{:.1$}", float, precision)
    } else {
        "".to_string()
    }
}

/// Creates an ISO8601 date string.
///
/// The returned string will be formatted as YYYY-MM-DD where YYYY is the 4 digit year, MM
/// is the month, and DD is the day in the month.
///
/// * `date` - the UTC date that will be converted.
///
pub fn fmt_yyyymmdd(date: &Date<Utc>) -> String {
    format!("{}", date.format("%Y-%m-%d"))
}
