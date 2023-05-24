//! # The list metadata command
//!
//! The command arguments allow the folling metadata to be reported.
//!
//! * list folders that have a specified filename.
//! * list folders that have a specified pathname.
//! * list the root folders that have been added.
//! * provide a summary of what metadata has been collected.
//!
#[cfg(windows)]
use std::path::{Component, Prefix, StripPrefixError};
use std::{fs, path::PathBuf};

use super::{
    commafy, mbufmt, rptcols, rptrow,
    text::{get_writer, write_strings, Report},
    Error, Metadata, Result, Session, StopWatch,
};
use clap::Args;

#[cfg(windows)]
/// Convert an error dealing with path prefixes into a CLI error.
impl From<StripPrefixError> for Error {
    fn from(error: StripPrefixError) -> Self {
        Error::from(format!("path: {error}"))
    }
}

/// The list folders command arguments.
#[derive(Args, Debug)]
pub struct CommandArgs {
    /// List folder contents that match a folders filename.
    #[clap(short, long, group = "cmd", requires = "folder", group = "folder_cmd")]
    name: bool,
    /// List the contents of a folder by its pathname.
    #[clap(short, long = "path", group = "cmd", requires = "folder", group = "folder_cmd")]
    pathname: bool,
    /// List the contents of the root folder(s).
    #[clap(long, group = "cmd")]
    root: bool,
    /// Show a summary of the collected file system information (default).
    #[clap(short, long = "info", group = "cmd")]
    info: bool,
    /// Show a list of files that had an error when loading.
    #[clap(long = "prob", group = "cmd")]
    problems: bool,
    /// Show the details of files, folders, and disk space used.
    #[clap(short = 'D', long = "details", group = "opts", conflicts_with_all = &["info", "problems"])]
    details: bool,
    /// Show a summary of the files, folders, and size of each folder.
    #[clap(short = 'S', long = "sum", group = "opts", conflicts_with_all = &["info", "problems"])]
    summary: bool,
    /// Recursively follow a folder structure.
    #[clap(short = 'R', long = "recurse", conflicts_with_all = &["info", "problems", "root"])]
    recurse: bool,
    /// The folder path or folder name to list
    #[clap(forbid_empty_values = true, value_name = "FOLDER", requires = "folder_cmd")]
    folder: Option<String>,
    #[clap(
        short = 'r', long = "rpt", value_name="FILE", forbid_empty_values = true,
        parse(try_from_str = super::parse_filename), group = "out"
    )]
    /// The report file pathname.
    pub report_path: Option<PathBuf>,
    /// Append to the report file, otherwise overwrite
    #[clap(short, long = "append", requires("out"))]
    pub append: bool,
}

/// The list folders command.
pub struct Command {
    /// The list folders command arguments.
    args: CommandArgs,
}

/// Several commands arguments share the same report output, consolidate it here.
#[derive(Debug, PartialEq)]
enum ReportType {
    /// Provides details about the contents of a folder.
    Detail,
    /// Provides `ls -l` style output.
    Information,
    /// Provides a summary about the contents of a folder.
    Summary,
}
impl ReportType {
    /// Generates a folder report based on the type of report.
    ///
    /// # Arguments
    ///
    /// * `folder_mds` is the collection of folder metadata.
    fn generate(&self, folder_mds: &Vec<Metadata>) -> Report {
        match self {
            ReportType::Detail => folders_details(&folder_mds),
            ReportType::Information => folders_information(&folder_mds),
            ReportType::Summary => folders_summary(&folder_mds),
        }
    }
}

impl Command {
    /// Creates an instance of the command.
    ///
    /// # Arguments
    ///
    /// * `args` the command arguments that will be used.
    pub fn new(args: CommandArgs) -> Command {
        Command { args }
    }

    /// Creates a [ReportType] report.
    ///
    /// # Arguments
    ///
    /// * `session` is the `domain` session used to get folder metadata.
    pub fn execute(self, session: &Session) -> Result<()> {
        let elapsed = StopWatch::start_new();
        let mut create_report = StopWatch::start_new();
        let report = if self.args.name {
            report_by_name(session, self.folder_name(), self.report_type(), self.args.recurse)?
        } else if self.args.pathname {
            let path = as_absolute_pathname(self.folder_name())?;
            report_by_pathname(session, &path, self.report_type(), self.args.recurse)?
        } else if self.args.root {
            report_root(session, self.report_type())?
        } else if self.args.problems {
            report_problems(session)?
        } else {
            report_summary(session)?
        };
        create_report.stop();

        let mut writer = get_writer(&self.args.report_path, self.args.append)?;
        // writetxt!(&mut writer, report.into_iter())?;
        write_strings(&mut writer, report.into_iter())?;
        log::info!("report create: {}, overall: {}", create_report.time_str(), elapsed.time_str());
        Ok(())
    }
    /// The folder name that will be reported.
    ///
    /// An empty string will be returned if the folder name has not been provided.
    fn folder_name(&self) -> &str {
        match &self.args.folder {
            Some(folder) => folder.as_str(),
            None => "",
        }
    }
    /// Creates the type of report based on the command arguments.
    fn report_type(&self) -> ReportType {
        match (self.args.details, self.args.summary) {
            (false, false) => ReportType::Information,
            (true, false) => ReportType::Detail,
            (false, true) => ReportType::Summary,
            _ => panic!("Yikes... Both detail and summary are true!"),
        }
    }
}

/// Generate a report based on a folders name.
///
/// # Arguments
///
/// * `session` will be used to get the folders metadata.
/// * `folder` is the folder name.
/// * `report_type` is the type of report to generate.
/// * `recurse` controls if the folder hierarchy should be included.
fn report_by_name(session: &Session, folder: &str, report_type: ReportType, recurse: bool) -> Result<Report> {
    let folder_mds = session.get_folder_by_name(folder, recurse)?;
    let report = report_type.generate(&folder_mds);
    Ok(report)
}

/// Generate a report based on the full pathname of a folder.
///
/// # Arguments
///
/// * `session` will be used to get the folders metadata.
/// * `path` is the folder pathname.
/// * `report_type` is the type of report to generate.
/// * `recurse` controls if the folder hierarchy should be included.
fn report_by_pathname(session: &Session, path: &str, report_type: ReportType, recurse: bool) -> Result<Report> {
    let folder_mds = session.get_folder_by_pathname(&path, recurse)?;
    let report = report_type.generate(&folder_mds);
    Ok(report)
}

/// Generate a report of the root folders.
///
/// # Arguments
///
/// * `session` will be used to get the root folders metadata.
/// * `report_type` is the type of report to generate.
fn report_root(session: &Session, report_type: ReportType) -> Result<Report> {
    let folder_mds = session.get_root_content()?;
    let report = report_type.generate(&folder_mds);
    Ok(report)
}

/// Generate a report describing information about the database.
///
/// # Arguments
///
/// * `session` will be used to get the database information.
fn report_summary(session: &Session) -> Result<Report> {
    let db_information = session.get_db_information()?;
    let mut report = Report::from(rptcols!(<=(2), >, >));
    let pathname = session.db();
    report.text(rptrow!(= "Database:", = pathname));
    report.text(rptrow!(_, = "Size:", = mbufmt!(db_information.database_size)));
    report.text(rptrow!(= "Root Folders:"));
    for folder in db_information.root_folders {
        report.text(rptrow!(_, = folder));
    }
    report.text(rptrow!(= "Row Counts:"));
    report.text(rptrow!(_, "Folders", commafy(db_information.folder_count)));
    report.text(rptrow!(_, "Files", commafy(db_information.file_count)));
    report.text(rptrow!(_, "Problems", commafy(db_information.problem_count)));
    Ok(report)
}

/// Generate a report of any problems that may have happened loading filesystem metadata.
///
/// # Arguments
///
/// * `session` will be used to get the problems metadata.
fn report_problems(session: &Session) -> Result<Report> {
    let metadatas = session.get_problems()?;
    // let report = reports::problems(&metadata);
    let mut report = Report::from(rptcols!(<=(2), =, =));
    for metadata in metadatas {
        if let Metadata::Folder(folder_md) = metadata {
            report.text(rptrow!(= &folder_md.pathname));
            for child in folder_md.children.values() {
                if let Metadata::Problem(problem) = child {
                    report.text(
                        // report_data!(_, format!("{}:", problem.name), &problem.description)
                        rptrow!(_, &problem.name, &problem.description),
                    );
                } else {
                    log::error!("Expected problem... {child:#?}")
                }
            }
        } else {
            log::error!("Expected folder... {metadata:#?}");
        }
    }
    Ok(report)
}

/// Create a report of summary information for a collection of folders.
///
/// The summary includes the following information for each sub folder.
/// * total number of files
/// * total number of subfolders
/// * total space taken by the folder
///
/// # Arguments
///
/// * `folder_mds` a collection of folder metadata to display.
fn folders_summary(folder_mds: &Vec<Metadata>) -> Report {
    let mut report = Report::from(rptcols!(>, >, >, =));
    report.header(rptrow!(^ "Files", ^ "Folders", ^ "Size", "Name"));
    let mut files: u64 = 0;
    let mut folders: u64 = 0;
    let mut size: u64 = 0;
    for metadata in folder_mds {
        let (md_files, md_folders, md_size) = folder_summary(&mut report, &metadata);
        files += md_files;
        folders += md_folders;
        size += md_size;
    }
    if folder_mds.len() > 1 {
        report.separator("=");
        report.text(rptrow!(mbufmt!(files), mbufmt!(folders), mbufmt!(size)));
    }
    report
}
/// Produce a summary of metadata information for a folder hierarchy.
///
/// A tuple is returned that contains the following summary information
///
/// * count of files
/// * count of subfolders
/// * total disk size used
///
/// # Arguments
///
/// * `metadata` the folder metadata that will be summarized.
/// * `report` is where summary information will be recorded.
fn folder_summary(report: &mut Report, metadata: &Metadata) -> (u64, u64, u64) {
    let mut files = 0;
    let mut folders = 0;
    let mut size = 0;
    if let Metadata::Folder(folder_md) = metadata {
        size += folder_md.size;
        for metadata in folder_md.children.values() {
            if metadata.is_folder() {
                folders += 1;
            } else {
                // files and symlinks are grouped together
                files += 1;
            }
            size += metadata.size();
        }
        report.text(rptrow!(mbufmt!(files), mbufmt!(folders), mbufmt!(size), folder_md.pathname.as_str()));
        for metadata in folder_md.children.values() {
            if metadata.is_folder() {
                let (child_files, child_folders, child_size) = folder_summary(report, metadata);
                files += child_files;
                folders += child_folders;
                size += child_size;
            }
        }
    } else {
        log::error!("'{}' is not a folder!!!", metadata.pathname());
    }
    (files, folders, size)
}

/// Creates a report that contains `ls -l` output.
///
/// # Arguments
///
/// * `folder_mds` the folder metadata that will be written.
fn folders_information(folder_mds: &Vec<Metadata>) -> Report {
    let mut report = Report::from(rptcols!(<, >, =, =));
    for metadata in folder_mds {
        folder_information(&mut report, &metadata);
    }
    report
}
/// Creates the `ls -l` report output for a folders metadata.
///
/// # Arguments
///
/// * `report` is where folder information will be recorded.
/// * `metadata` the folder metadata that will be written.
fn folder_information(report: &mut Report, metadata: &Metadata) {
    if let Metadata::Folder(folder_md) = metadata {
        report.text(rptrow!(= metadata.pathname()));
        for child_md in folder_md.children.values() {
            let file_type = if child_md.is_folder() {
                "DIR"
            } else if metadata.is_symlink() {
                "SYM"
            } else {
                "FILE"
            };
            let size = mbufmt!(child_md.size());
            use toolslib::date_time::get_local_ts;
            use chrono::{Datelike, Local};
            let modified_dt = get_local_ts(child_md.modified() as i64);
            let modified_fmt = if modified_dt.year() == Local::now().year() { "%h %_d %H:%M" } else { "%h %_d %Y" };
            let date = modified_dt.format(modified_fmt).to_string();
            report.text(rptrow!(file_type, size, date, child_md.name()));
        }
        for child_md in folder_md.children.values() {
            if child_md.has_childen() {
                folder_information(report, child_md);
            }
        }
    } else {
        log::error!("'{}' is not a folder!!! {metadata:#?}", metadata.pathname());
    }
}

/// Create a report with details about a folders output.
///
/// # Arguments
///
/// * `folder_mds` the folder metadata that will be written.
fn folders_details(folder_mds: &Vec<Metadata>) -> Report {
    let mut total_files: u64 = 0;
    let mut total_folders: u64 = 0;
    let mut total_size: u64 = 0;
    let mut report = Report::from(rptcols!(>+(7), >+(7), >+(7), =));
    report.header(rptrow!("Files", "Folders", "Size", "Folder Name"));
    for metadata in folder_mds {
        let (files, folders, size) = folder_detail(&mut report, &metadata);
        total_files += files;
        total_folders += folders;
        total_size += size;
    }
    report.separator("=").text(rptrow!(mbufmt!(total_files), mbufmt!(total_folders), mbufmt!(total_size),));
    report
}
/// Create the detail output of a folders content.
///
/// # Arguments
///
/// * `report` is where folder information will be recorded.
/// * `metadata` is the folder metadata that will be used.
fn folder_detail(report: &mut Report, metadata: &Metadata) -> (u64, u64, u64) {
    let mut files = 0 as u64;
    let mut folders = 0 as u64;
    let mut size = metadata.size();
    if let Metadata::Folder(folder_md) = metadata {
        folder_md.children.values().for_each(|md| {
            if md.is_folder() {
                folders += 1;
            } else if md.is_file() || md.is_symlink() {
                files += 1;
            }
        });
        report.text(rptrow!(commafy(files), commafy(folders), mbufmt!(size), metadata.pathname()));
        for metadata in folder_md.children.values() {
            if let Metadata::File(md) = metadata {
                size += md.size;
                report.text(rptrow!(_, _, mbufmt!(md.size), md.name.as_str()));
            }
        }
        for metadata in folder_md.children.values() {
            if metadata.is_folder() {
                let (child_files, child_folders, child_size) = folder_detail(report, metadata);
                files += child_files;
                folders += child_folders;
                size += child_size;
            }
        }
    } else {
        log::error!("Metadata ({}) is not a folder!!!", metadata.pathname());
    }
    (files, folders, size)
}

/// A helper function that creates an absolute pathname from some folder.
///
/// The function will accept `.` and `..` directory names. In this case
/// the current directory or parent directory name will be returned.
///
/// On *Windoz* the pathname will always contain a leading drive letter. If
/// missing it will prepend the current directory drive letter to the pathname.
///
/// # Arguments
///
/// * `folder_name` the folder name that will be converted to an absolute pathname.
fn as_absolute_pathname(folder_name: &str) -> Result<String> {
    // the windows configuration requires this to be mut
    #[allow(unused_mut)]
    let mut absolute_path = if folder_name.len() == 0 {
        fs::canonicalize(".")?
    } else if folder_name == "." || folder_name == ".." {
        fs::canonicalize(PathBuf::from(folder_name))?
    } else {
        let mut folder_path = PathBuf::from(folder_name);
        if !folder_path.is_absolute() {
            match fs::canonicalize(folder_path) {
                Ok(path) => folder_path = path,
                Err(_) => return Err(Error::from(format!("'{}' must be an absolute pathname...", folder_name))),
            }
        }
        folder_path
    };
    #[cfg(windows)]
    {
        let mut components = absolute_path.components();
        let component = components.next().ok_or(Error::from("Yikes! Path had no components..."))?;
        if let Component::Prefix(prefix_component) = component {
            let drive = match prefix_component.kind() {
                Prefix::VerbatimDisk(drive) => format!("{}:", drive as char),
                Prefix::Disk(drive) => format!("{}:", drive as char),
                _ => String::default(),
            };
            absolute_path = PathBuf::from(drive).join(components.as_path());
        }
    }
    Ok(absolute_path.display().to_string())
}
