//! # A text based report generator
//!
//! The intent of this module is to provide a common text based reporting engine.
//! There was so much commonality between the various cli reporting commands it
//! seemed reasonable to build a common reporting engine.
//!
//! The components allow text to be placed into report columns, abstracting how the text is
//! really generated. At some point I would think defining a set of macros to help genearate
//! the output will be in order.

use std::{cmp, fs, fmt, io, path::PathBuf, result};

use chrono::prelude::*;

/// The text module result.
type Result<T> = result::Result<T, Error>;

/// The text Error that can be captured outside the module.
///
/// Currently it contains only a String but can be extended to an enum later on.
#[derive(Debug)]
pub struct Error(String);

/// Allow the error to be use in format!, writeln!, etc.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Creates an error from a String.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(error.as_str())
    }
}

/// Creates an error from a str slice.
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(format!("text: {error}"))
    }
}

/// Creates an error from a str slice.
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error(format!("text: {error}"))
    }
}

/// Convert the error into a String.
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.0
    }
}

/// Indicate what alignment a text column will have.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Alignment {
    /// Text will be aligned on the left hand side of a column.
    Left,
    /// Text will be centered in a column.
    Center,
    /// Text will be aligned on the right hand side of a column.
    Right,
    /// Text will be left justified and not have any padding applied to it.
    AsIs,
}

/// Let Alignment be used in format!, writeln!, etc.
impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alignment = match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
            Self::AsIs => "None",
        };
        write!(f, "{alignment}")
    }
}

/// The description of a column in a report
#[derive(Debug)]
pub struct ColumnDescription {
    /// The desired width of a report column.
    pub width: usize,
    /// The default alignment of text for a report column.
    pub default_alignment: Alignment,
}

/// Construct the column description based on an alignment.
impl From<Alignment> for ColumnDescription {
    fn from(alignment: Alignment) -> Self {
        ColumnDescription {
            width: 0,
            default_alignment: alignment,
        }
    }
}

impl ColumnDescription {
    /// Allows the column width to be initialized when constructing the instance.
    pub fn with_width(mut self, minimum_width: usize) -> Self {
        self.width = minimum_width;
        self
    }
}

/// Establish a container for the column descriptions.
pub struct ColumnDescriptions(Vec<ColumnDescription>);

/// Convert an array of alignments to an array of column descriptions.
impl From<Vec<Alignment>> for ColumnDescriptions {
    fn from(alignments: Vec<Alignment>) -> Self {
        let column_descriptions = alignments
            .iter()
            .map(|alignment| ColumnDescription::from(*alignment))
            .collect();
        ColumnDescriptions(column_descriptions)
    }
}

impl From<Vec<ColumnDescription>> for ColumnDescriptions {
    fn from(column_descriptions: Vec<ColumnDescription>) -> Self {
        ColumnDescriptions(column_descriptions)
    }
}

// /// The helper functions for the column descriptions container.
// impl ColumnDescriptions {
//     /// Allows the column descriptions to be initialized from a vector.
//     pub fn new(column_descriptions: Vec<ColumnDescription>) -> Self {
//         ColumnDescriptions(column_descriptions)
//     }
// }

/// The description of a row in a report
#[derive(Debug)]
pub struct RowDescription(Vec<ColumnDescription>);

/// Creates the description of a row from the column descriptions.
impl From<ColumnDescription> for RowDescription {
    fn from(column: ColumnDescription) -> Self {
        RowDescription(vec![column])
    }
}

/// Creates the description of a row from the column descriptions.
impl From<Vec<ColumnDescription>> for RowDescription {
    fn from(columns: Vec<ColumnDescription>) -> Self {
        RowDescription(columns)
    }
}

/// Creates the row description from the column descriptions container.
impl From<ColumnDescriptions> for RowDescription {
    fn from(column_descriptions: ColumnDescriptions) -> Self {
        RowDescription(column_descriptions.0)
    }
}

impl RowDescription {
    /// Get the number of columns that have been defined.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Add columns description to an existing row description.
    pub fn add_columns(&mut self, mut columns: ColumnDescriptions) -> &mut Self {
        self.0.append(&mut columns.0);
        self
    }

    /// Get the description of a column.
    pub fn get_column_description(&self, column: usize) -> Option<&ColumnDescription> {
        self.0.get(column)
    }
    /// Formats each row column based on the associated column description.
    pub fn format_row(&self, row: &Row) -> Result<Vec<String>> {
        if !row.id.is_separator() && row.len() != self.len() {
            Err(Error::from(format!("Row column count ({}) != Row description count ({})", row.len(), self.len())))
        } else {
            let mut columns = vec![];
            if let RowID::Separator(separator) = row.id {
                let separator = separator.to_string();
                for column_description in &self.0 {
                    columns.push(separator.repeat(column_description.width));
                }
            } else {
                for i in 0..self.len() {
                    let column = &row.columns[i];
                    let column_description = &self.0[i];
                    // don't bother aligining text if it is the same size or larger than the description width
                    let column_text = if column_description.width == 0 || column.text.len() >= column_description.width
                    {
                        column.text.clone()
                    } else {
                        // if the column does not specify an alignment use the description default
                        let column_alignment = match column.alignment {
                            Some(alignment) => alignment,
                            _ => column_description.default_alignment,
                        };
                        align_text(&column.text, column_description.width, column_alignment)
                    };
                    columns.push(column_text);
                }
            }
            Ok(columns)
        }
    }
}

/// A container for text in a column.
///
/// The column text is not formatted until the report is generated. Leading
/// and trailing white space *will NOT* be trimmed during report generation.
#[derive(Debug)]
pub struct Column {
    /// The column text.
    pub text: String,
    /// The text alignment.
    ///
    /// If the alignment is *None*, alignment will come from the column property default.
    pub alignment: Option<Alignment>,
}

impl Column {
    /// Allow the column alignment to be set when constructing.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

/// Create a column from a string.
impl From<String> for Column {
    fn from(text: String) -> Self {
        Column { text, alignment: None }
    }
}

/// Create a column from a string slice.
impl From<&str> for Column {
    fn from(text: &str) -> Self {
        Column::from(String::from(text))
    }
}

/// Create a container for a collection of row columns.
// TODO: add the ability to set the column alignments in bulk and individually???
pub struct Columns(Vec<Column>);

/// Create the column container from a collection of string slices.
impl From<Vec<&str>> for Columns {
    fn from(columns_text: Vec<&str>) -> Self {
        let columns = columns_text.iter().map(|column| Column::from(*column)).collect();
        Columns(columns)
    }
}

/// Create the column container from a collection of strings.
impl From<Vec<String>> for Columns {
    fn from(columns_text: Vec<String>) -> Self {
        let columns = columns_text.iter().map(|text| Column::from(text.as_str())).collect();
        Columns(columns)
    }
}

/// Identifies the type of a line in the report.
#[derive(Debug, PartialEq)]
pub enum RowID {
    /// The report row is a header.
    Header,
    /// The report row contains details.
    Detail,
    /// The report row columns will consist of the separator character.
    Separator(char),
}

impl RowID {
    /// A helper that identifies the row id as a separator.
    pub fn is_separator(&self) -> bool {
        match self {
            Self::Header | Self::Detail => false,
            _ => true,
        }
    }
}

/// A row that will be generated by the report.
#[derive(Debug)]
pub struct Row {
    id: RowID,
    columns: Vec<Column>,
    /// The default text alignment for columns added to the row.
    ///
    /// If the alignment is *None*, alignment will come from the column description default.
    alignment: Option<Alignment>,
}

/// Create a row with the specified row identifier.
impl From<RowID> for Row {
    fn from(row_type: RowID) -> Self {
        Row {
            id: row_type,
            columns: vec![],
            alignment: None,
        }
    }
}

impl Row {
    /// Add a column to the row.
    pub fn add(&mut self, mut column: Column) -> &mut Self {
        if let Some(default_alignment) = self.alignment {
            if column.alignment.is_none() {
                column.alignment = Some(default_alignment);
            }
        }
        self.columns.push(column);
        self
    }
    /// Add a collection of columns to the row.
    pub fn add_columns(&mut self, columns: Columns) -> &mut Self {
        // self.columns.append(&mut columns.0);
        for column in columns.0 {
            self.add(column);
        }
        self
    }
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
    /// Allows a column to be added when the row is being created.
    pub fn with_column(mut self, column: Column) -> Self {
        self.add(column);
        self
    }
    /// Allows a collection of columns to be added with the row is being created.
    pub fn with_columns(mut self, columns: Columns) -> Self {
        self.add_columns(columns);
        self
    }
    /// The number of columns contained in the row.
    pub fn len(&self) -> usize {
        self.columns.len()
    }
    /// Get a column from the row.
    pub fn get_column(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }
}

/// The public facing builder to create text reports.
pub struct Report {
    /// The report row column description.
    pub row_description: RowDescription,
    /// The rows of the report.
    pub content: Vec<Row>,
    /// Adjust the RowDescription width for each Row added to the report.
    pub auto_size: bool,
}

/// Creates a new instance of the report from a row description.
///
impl From<RowDescription> for Report {
    fn from(row_description: RowDescription) -> Self {
        Report {
            row_description,
            content: vec![],
            auto_size: true,
        }
    }
}

/// Provide a short cut to creating the report from column descriptions.
impl From<Vec<ColumnDescription>> for Report {
    fn from(columns_descriptions: Vec<ColumnDescription>) -> Self {
        Report::from(RowDescription::from(columns_descriptions))
    }
}

impl From<Vec<Alignment>> for Report {
    fn from(alignments: Vec<Alignment>) -> Self {
        Report::from(RowDescription::from(ColumnDescriptions::from(alignments)))
    }
}

impl Report {
    /// Allows the report to autosize columns in the report or not.
    pub fn with_autosize(mut self, auto_size: bool) -> Self {
        self.auto_size = auto_size;
        self
    }
    /// Add a row to the report.
    pub fn add(&mut self, row: Row) -> Result<&mut Self> {
        if !row.id.is_separator() {
            if self.row_description.len() != row.len() {
                let mismatch = format!(
                    "Row columns ({}) do not match row description columns ({})",
                    row.len(),
                    self.row_description.len()
                );
                return Err(Error::from(mismatch));
            }
            if self.auto_size {
                for column in 0..self.row_description.len() {
                    unsafe {
                        let description = self.row_description.0.get_unchecked_mut(column);
                        description.width = cmp::max(description.width, row.columns.get_unchecked(column).text.len())
                    }
                }
            }
        }
        self.content.push(row);
        Ok(self)
    }
    /// Add a collection of rows to the report.
    pub fn with_rows(mut self, rows: Vec<Row>) -> Result<Self> {
        for row in rows {
            self.add(row)?;
        }
        Ok(self)
    }

    pub fn generate(&self, mut writer: Box<dyn io::Write>) -> Result<()> {
        for row in &self.content {
            let text = self.row_description.format_row(row)?.join(" ");
            writer.write_all(text.as_bytes())?;
            writer.write_all("\n".as_bytes())?;
        }
        Ok(())
    }

}

/// Allows the report to be converted to an iterator that returns a reference to the
/// report contents.
impl<'r> IntoIterator for &'r Report {
    type Item = String;
    /// The iterator implementation for a report generator.
    type IntoIter = Reporter<'r>;
    /// Creates the report builder iterator.
    fn into_iter(self) -> Self::IntoIter {
        Reporter {
            report: self,
            row_index: 0,
        }
    }
}
/// The report builder iterator.
pub struct Reporter<'r> {
    /// A reference to the report container.
    report: &'r Report,
    /// The report row returned when `next` is called.
    row_index: usize,
}

/// The report builder iterator responsible for creating the records of a report.
impl<'r> Iterator for Reporter<'r> {
    type Item = String;
    /// For each report record, columns are formatted and then joined to create the line of text
    /// written to report.
    fn next(&mut self) -> Option<Self::Item> {
        let mut string_option = None;
        if let Some(row) = self.report.content.get(self.row_index) {
            if let Ok(formatted_columns) = self.report.row_description.format_row(row) {
                string_option = Some(formatted_columns.join(" "));
                self.row_index += 1;
            }
        }
        string_option
    }
}

/// Gets a `io::Write` writer for either a file or `stdout`.
/// 
/// # Arguments
/// 
/// * `file_option` - if `None` then `stdout` will be used otherwise the file path will be opened.
/// * `append` - if writing to a file, append output if `true` otherwise truncate existing file contents.
pub fn get_writer(file_option: &Option<PathBuf>, append: bool) -> Result<Box<dyn io::Write>> {
    let writer = if let Some(file_path) = &file_option {
        let mut open_options = fs::OpenOptions::new();
        if append {
            open_options.append(true);
        } else {
            open_options.write(true).truncate(true).create(true);
        }
        let filename = format!("{}", file_path.as_path().display());
        Box::new(open_options.open(filename)?) as Box<dyn io::Write>
    } else {
        Box::new(io::stdout()) as Box<dyn io::Write>
    };
    Ok(writer)
}

/// Writes a report to either stdout or a file.
///
pub struct ReportWriter(Option<PathBuf>);

impl ReportWriter<> {
    /// Creates an instance of the report writer container.
    ///
    /// # Arguments
    ///
    /// * `pathbuf_option` - the optional file pathname where generated reports will be written.
    ///
    pub fn new(pathbuf_option: Option<PathBuf>) -> ReportWriter {
        ReportWriter(pathbuf_option)
    }

    /// Creates `Write` instance where reports can be written.
    ///
    /// If the report writer contains a file pathname, an error can occur due to permission
    /// or locking issues.
    pub fn generate(&self, report: &Report, append: bool) -> Result<()> {
        const NL: &[u8] = "\n".as_bytes();
        let mut writer = get_writer(&self.0, append)?;
        for row in report.into_iter() {
            writer.write_all(row.as_bytes())?;
            writer.write_all(NL)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Date, NaiveDate, Utc};

    #[test]
    fn error() {
        assert_eq!(Error::from("foo").0, "text: foo");
        assert_eq!(Error::from(String::from("bar")).0, String::from("text: bar"));
        assert_eq!(String::from(Error::from("foobar")), String::from("text: foobar"));
        assert_eq!(format!("{}", Error::from("raboof")), format!("text: raboof"));
    }

    #[test]
    fn alignment() {
        assert_eq!(format!("{}", Alignment::Left), String::from("Left"));
        assert_eq!(format!("{}", Alignment::Center), String::from("Center"));
        assert_eq!(format!("{}", Alignment::Right), String::from("Right"));
        assert_eq!(format!("{}", Alignment::AsIs), String::from("None"));
    }

    #[test]
    fn column_description() {
        let testcase = ColumnDescription::from(Alignment::Left);
        assert_eq!(testcase.width, 0);
        assert_eq!(testcase.default_alignment, Alignment::Left);
        let testcase = ColumnDescription::from(Alignment::Left).with_width(5);
        assert_eq!(testcase.width, 5);
        assert_eq!(testcase.default_alignment, Alignment::Left);
        let testcase = ColumnDescription::from(Alignment::Center);
        assert_eq!(testcase.width, 0);
        assert_eq!(testcase.default_alignment, Alignment::Center);
        let testcase = ColumnDescription::from(Alignment::Right);
        assert_eq!(testcase.width, 0);
        assert_eq!(testcase.default_alignment, Alignment::Right);
    }

    #[test]
    fn column_descriptions() {
        let testcase =
            ColumnDescriptions::from(vec![Alignment::AsIs, Alignment::Right, Alignment::Left, Alignment::Center]);
        assert_eq!(testcase.0.len(), 4);
        assert_eq!(testcase.0[0].default_alignment, Alignment::AsIs);
        assert_eq!(testcase.0[1].default_alignment, Alignment::Right);
        assert_eq!(testcase.0[2].default_alignment, Alignment::Left);
        assert_eq!(testcase.0[3].default_alignment, Alignment::Center);
    }

    #[test]
    fn row_description() {
        let testcase = RowDescription::from(vec![]);
        assert_eq!(testcase.len(), 0);
        let testcase = RowDescription::from(vec![
            ColumnDescription::from(Alignment::Left).with_width(5),
            ColumnDescription::from(Alignment::Center).with_width(10),
            ColumnDescription::from(Alignment::Right),
        ]);
        let column = testcase.get_column_description(0).unwrap();
        assert_eq!(column.default_alignment, Alignment::Left);
        assert_eq!(column.width, 5);
        let column = testcase.get_column_description(1).unwrap();
        assert_eq!(column.default_alignment, Alignment::Center);
        assert_eq!(column.width, 10);
        let column = testcase.get_column_description(2).unwrap();
        assert_eq!(column.default_alignment, Alignment::Right);
        assert_eq!(column.width, 0);
        assert!(testcase.get_column_description(4).is_none());
        let mut testcase = RowDescription::from(ColumnDescriptions::from(vec![Alignment::Left]));
        let column = testcase.get_column_description(0).unwrap();
        assert_eq!(column.default_alignment, Alignment::Left);
        assert_eq!(column.width, 0);
        assert!(testcase.get_column_description(1).is_none());
        testcase.add_columns(ColumnDescriptions::from(vec![Alignment::AsIs]));
        assert_eq!(testcase.len(), 2);
        let column = testcase.get_column_description(1).unwrap();
        assert_eq!(column.default_alignment, Alignment::AsIs);
    }

    #[test]
    fn row_descriptions() {
        let testcase =
            ColumnDescriptions::from(vec![Alignment::AsIs, Alignment::Right, Alignment::Center, Alignment::Left]);
        assert_eq!(testcase.0.len(), 4);
        assert_eq!(testcase.0[0].default_alignment, Alignment::AsIs);
        assert_eq!(testcase.0[0].width, 0);
        assert_eq!(testcase.0[1].default_alignment, Alignment::Right);
        assert_eq!(testcase.0[1].width, 0);
        assert_eq!(testcase.0[2].default_alignment, Alignment::Center);
        assert_eq!(testcase.0[2].width, 0);
        assert_eq!(testcase.0[3].default_alignment, Alignment::Left);
        assert_eq!(testcase.0[2].width, 0);
    }

    #[test]
    fn column() {
        let testcase = Column::from("foobar");
        assert_eq!(testcase.text, String::from("foobar"));
        assert_eq!(testcase.alignment, None);
        let testcase = Column::from(String::from("left")).with_alignment(Alignment::Left);
        assert_eq!(testcase.text, String::from("left"));
        assert_eq!(testcase.alignment, Some(Alignment::Left));
        let testcase = Column::from("center").with_alignment(Alignment::Center);
        assert_eq!(testcase.text, String::from("center"));
        assert_eq!(testcase.alignment, Some(Alignment::Center));
        let testcase = Column::from("right").with_alignment(Alignment::Right);
        assert_eq!(testcase.text, String::from("right"));
        assert_eq!(testcase.alignment, Some(Alignment::Right));
    }

    #[test]
    fn columns() {
        let testcase = Columns::from(vec!["foo", "bar"]);
        assert_eq!(testcase.0.len(), 2);
        assert_eq!(testcase.0[0].text, String::from("foo"));
        assert_eq!(testcase.0[0].alignment, None);
        assert_eq!(testcase.0[1].text, String::from("bar"));
        assert_eq!(testcase.0[1].alignment, None);
        let testcase = Columns::from(vec![String::from("bar"), String::from("foo")]);
        assert_eq!(testcase.0.len(), 2);
        assert_eq!(testcase.0[0].text, String::from("bar"));
        assert_eq!(testcase.0[0].alignment, None);
        assert_eq!(testcase.0[1].text, String::from("foo"));
        assert_eq!(testcase.0[1].alignment, None);
    }

    #[test]
    fn row_id() {
        assert!(!RowID::Header.is_separator());
        assert!(!RowID::Detail.is_separator());
        assert!(RowID::Separator('-').is_separator());
    }

    #[test]
    fn row() {
        let testcase = Row::from(RowID::Header);
        assert_eq!(testcase.len(), 0);
        assert_eq!(testcase.id, RowID::Header);
        assert_eq!(Row::from(RowID::Detail).id, RowID::Detail);
        assert_eq!(Row::from(RowID::Separator('-')).id, RowID::Separator('-'));
        let mut testcase = Row::from(RowID::Detail)
            .with_column(Column::from("zero"))
            .with_columns(Columns::from(vec!["one"]));
        assert_eq!(testcase.len(), 2);
        assert_eq!(testcase.get_column(0).unwrap().text, String::from("zero"));
        assert_eq!(testcase.get_column(1).unwrap().text, String::from("one"));
        testcase
            .add(Column::from("two"))
            .add_columns(Columns::from(vec!["three"]));
        assert_eq!(testcase.get_column(2).unwrap().text, String::from("two"));
        assert_eq!(testcase.get_column(3).unwrap().text, String::from("three"));
        assert!(testcase.get_column(4).is_none());
    }

    #[test]
    fn format_row() {
        let row_description = RowDescription::from(vec![
            ColumnDescription::from(Alignment::Right).with_width(5),
            ColumnDescription::from(Alignment::Center).with_width(5),
            ColumnDescription::from(Alignment::Left).with_width(5),
        ]);
        let row = Row::from(RowID::Separator('*'));
        let columns = row_description.format_row(&row).unwrap();
        assert_eq!(columns.len(), 3);
        assert_eq!(columns, vec!["*****", "*****", "*****"]);
        let columns = row_description
            .format_row(&Row::from(RowID::Detail).with_columns(Columns(vec![
                Column::from("foo").with_alignment(Alignment::Left),
                Column::from("foo"),
                Column::from("foo").with_alignment(Alignment::Right),
            ])))
            .unwrap();
        assert_eq!(columns, vec!["foo  ", " foo ", "  foo"]);
    }

    #[test]
    fn report() {
        let mut testcase = Report::from(RowDescription::from(vec![
            ColumnDescription::from(Alignment::Left).with_width(5),
            ColumnDescription::from(Alignment::Center).with_width(5),
            ColumnDescription::from(Alignment::Right).with_width(5),
        ]));
        assert!(testcase.add(Row::from(RowID::Header)).is_err());
        assert!(testcase
            .add(Row::from(RowID::Detail).with_columns(Columns::from(vec!["one", "two", "three", "four",])))
            .is_err());
        testcase
            .add(Row::from(RowID::Header).with_columns(Columns::from(vec!["header1", "header2", "header3"])))
            .unwrap();
        testcase
            .add(Row::from(RowID::Detail).with_columns(Columns::from(vec!["detail1", "detail2", "detail3"])))
            .unwrap();
        testcase.add(Row::from(RowID::Separator('-'))).unwrap();
        assert_eq!(testcase.content.len(), 3);
        assert_eq!(testcase.content[0].id, RowID::Header);
        assert_eq!(testcase.content[1].id, RowID::Detail);
        if let RowID::Separator(separator) = testcase.content[2].id {
            assert_eq!(separator, '-');
        } else {
            panic!("did not find a RowID::Separator row!!!");
        }
    }

    #[test]
    fn autosize_report() {
        let testcase = Report::from(RowDescription::from(vec![ColumnDescription::from(Alignment::Left)]))
            .with_autosize(false)
            .with_rows(vec![Row::from(RowID::Detail).with_columns(Columns::from(vec!["text"]))])
            .unwrap();
        assert!(!testcase.auto_size);
        assert_eq!(testcase.row_description.get_column_description(0).unwrap().width, 0);
        let mut testcase = Report::from(RowDescription::from(vec![ColumnDescription::from(Alignment::Left)]))
            .with_rows(vec![Row::from(RowID::Separator('-'))])
            .unwrap();
        assert!(testcase.auto_size);
        assert_eq!(testcase.row_description.get_column_description(0).unwrap().width, 0);
        testcase
            .add(Row::from(RowID::Header).with_columns(Columns::from(vec!["header"])))
            .unwrap();
        assert_eq!(testcase.row_description.get_column_description(0).unwrap().width, 6);
        testcase
            .add(Row::from(RowID::Detail).with_columns(Columns::from(vec!["row details"])))
            .unwrap();
        assert_eq!(testcase.row_description.get_column_description(0).unwrap().width, 11);
        testcase
            .add(Row::from(RowID::Detail).with_columns(Columns::from(vec!["details"])))
            .unwrap();
        assert_eq!(testcase.row_description.get_column_description(0).unwrap().width, 11);
    }

    #[test]
    fn reporter() {
        let report =
            Report::from(RowDescription::from(ColumnDescriptions::from(vec![Alignment::Left, Alignment::Left])))
                .with_autosize(false)
                .with_rows(vec![
                    Row::from(RowID::Detail).with_columns(Columns::from(vec!["foo", "bar"])),
                    Row::from(RowID::Detail).with_columns(Columns::from(vec!["foobar", "raboof"])),
                ])
                .unwrap();
        let mut iter = report.into_iter();
        assert_eq!(iter.next().unwrap(), "foo bar");
        assert_eq!(iter.next().unwrap(), "foobar raboof");
        assert!(iter.next().is_none());
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

    #[test]
    fn fmt_isodate_util() {
        let date: Date<Utc> = Date::from_utc(NaiveDate::from_ymd(2022, 10, 5), Utc);
        assert_eq!(fmt_isodate(&date), "2022-10-05");
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
        Alignment::AsIs => String::from(value),
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
pub fn fmt_isodate(date: &Date<Utc>) -> String {
    format!("{}", date.format("%Y-%m-%d"))
}
