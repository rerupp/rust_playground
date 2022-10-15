//! # The list metadata command
//!
//! The command arguments allow the folling metadata to be reported.
//!
//! * list folders that have a specified filename.
//! * list folders that have a specified pathname.
//! * list the root folders that have been added.
//! * provide a summary of what metadata has been collected.
//!
use std::path::{Component, PathBuf};
#[cfg(windows)]
use std::path::{Prefix, StripPrefixError};
use std::{fs, io};

use toolslib::stopwatch::StopWatch;

use clap::Args;

use super::*;

/// Convert an IO Error into a CLI error.
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::from(format!("io: {error}"))
    }
}

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
    #[clap(short, long, group = "cmd")]
    root: bool,
    /// Show a summary of the collected file system information (default).
    #[clap(short, long = "info", group = "cmd")]
    info: bool,
    /// Show a list of files that had an error when loading.
    #[clap(short = 'P', long = "prob", group = "cmd")]
    problems: bool,
    /// Show a summary of the files, folders, and size of each folder.
    #[clap(short = 'S', long = "sum", conflicts_with_all = &["info", "problems"])]
    summary: bool,
    /// Recursively follow a folder structure.
    #[clap(short = 'R', long = "recurse", conflicts_with_all = &["info", "problems", "root"])]
    recurse: bool,
    /// The folder path or folder name to list
    #[clap(forbid_empty_values = true, value_name = "FOLDER", requires = "folder_cmd")]
    folder: Option<String>,
}

/// The list folders command.
pub struct Command {
    /// The list folders command arguments.
    args: CommandArgs,
}

use writers::write_db_information;
use writers::write_folders;
use writers::write_problems;
use writers::write_summary;

impl Command {
    /// Creates an instance of the command.
    ///
    /// # Arguments
    ///
    /// * `args` - the command arguments that will be used.
    pub fn new(args: CommandArgs) -> Command {
        Command { args }
    }

    /// Uses a [Session] from `fsviewlib` to call the API that will add folder metadata to the database.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    pub fn execute(self, session: &Session) -> Result<()> {
        let elapsed = StopWatch::start_new();
        if self.args.name {
            self.execute_name(session)?;
        } else if self.args.pathname {
            self.execute_pathname(session)?;
        } else if self.args.root {
            self.execute_root(session)?;
        } else if self.args.problems {
            // let problem_md = session.get_problems()?;
            // write_problems(problem_md, None)?;
            self.execute_problems(session)?;
        } else {
            // let db_information = session.get_db_information()?;
            // write_db_information(&format!("{session}"), db_information, None)?;
            self.execute_summary(session)?;
        }
        log::info!("elapased time: {}", elapsed.time_str());
        Ok(())
    }

    /// Executes the list command that includes a folder name (`--name`) argument.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    fn execute_name(self, session: &Session) -> Result<()> {
        let folder = self.args.folder.unwrap();
        let folder_metadata = session.get_folder_by_name(&folder, self.args.recurse)?;
        if self.args.summary {
            write_summary(folder_metadata, None, false)
        } else {
            write_folders(folder_metadata, None, false)
        }
    }

    /// Executes the list command that includes a pathname (`--path`) argument.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    fn execute_pathname(self, session: &Session) -> Result<()> {
        let folder_pathname = as_absolute_pathname(&self.args.folder.unwrap())?;
        let folder_metadata = session.get_folder_by_pathname(&folder_pathname, self.args.recurse)?;
        if self.args.summary {
            write_summary(folder_metadata, None, false)
        } else {
            write_folders(folder_metadata, None, false)
        }
    }

    /// Executes the list command that includes a root (`--root`) argument.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    fn execute_root(self, session: &Session) -> Result<()> {
        let folder_metadata = session.get_root_content()?;
        if self.args.summary {
            write_summary(folder_metadata, None, false)
        } else {
            write_folders(folder_metadata, None, false)
        }
    }

    /// Executes the list command that includes a problems (`--prob`) argument.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    fn execute_problems(self, session: &Session) -> Result<()> {
        let problem_md = session.get_problems()?;
        write_problems(problem_md, None, false)?;
        Ok(())
    }

    /// Executes the list command that includes a summary (`--sum`) argument.
    ///
    /// # Arguments
    ///
    /// * `session` - the `domain` session that will be used to add the metadata.
    fn execute_summary(self, session: &Session) -> Result<()> {
        let db_information = session.get_db_information()?;
        write_db_information(&format!("{session}"), db_information, None, false)?;
        Ok(())
    }
}

mod writers {
    //! An internal set of utilities that help produce the text data reports.
    use super::{DbInformation, Error, Metadata, Result};
    use chrono::{Datelike, Local, TimeZone};
    use std::{fmt, fmt::Write as FmtWrite, io, path::PathBuf};
    use thousands::Separable;
    use toolslib::text;

    /// The format error can only come from here right now, so isolate the conversion
    /// here.
    impl From<fmt::Error> for Error {
        fn from(error: std::fmt::Error) -> Self {
            Error::from(format!("{error}"))
        }
    }

    /// The text error can only come from here right now, so isolate the conversion
    /// here.
    impl From<text::Error> for Error {
        fn from(error: text::Error) -> Self {
            Error(String::from(error))
        }
    }

    /// A function that produces `ls -l` style text output.
    /// 
    /// # Arguments
    /// 
    /// * `folder_metadata` - a collection of folder metadata to display.
    /// * `file_option` - the optional file pathname for text output.
    /// * `append` - if writing to a file, append text otherwise truncate existing content.
    pub fn write_folders(folder_metadata: Vec<Metadata>, file_option: Option<PathBuf>, append: bool) -> Result<()> {
        let mut writer = text::get_writer(&file_option, append)?;
        for metadata in folder_metadata {
            write_folder(&mut writer, &metadata)?;
        }
        Ok(())
    }
    /// A function that writes `ls -l` style text output for a folders metadata
    /// 
    /// # Arguments
    /// 
    /// * `writer` - the target for text output.
    /// * `metadata` - the folder metadata that will be written.
    fn write_folder(writer: &mut Box<dyn io::Write>, metadata: &Metadata) -> Result<()> {
        if let Metadata::Folder(folder_md) = metadata {
            let mut content = format!("\n{}\n", metadata.pathname());
            for child_md in &folder_md.children {
                let file_type = if child_md.is_folder() {
                    "DIR"
                } else if metadata.is_symlink() {
                    "SYM"
                } else {
                    "FILE"
                };
                let size = to_pretty_size(child_md.size());
                let date = to_datetime_string(child_md.modified());
                write!(content, "\n{:<4} {:>7} {} {}", file_type, size, date, child_md.name())?;
            }
            writeln!(writer, "{content}")?;

            for child_md in &folder_md.children {
                if has_children(child_md) {
                    write_folder(writer, child_md)?;
                }
            }
        }

        Ok(())
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
    /// * `folder_metadata` - a collection of folder metadata to display.
    /// * `file_option` - the optional file pathname for text output.
    /// * `append` - if writing to a file, append text otherwise truncate existing content.
    pub fn write_summary(folder_metadata: Vec<Metadata>, file_option: Option<PathBuf>, append: bool) -> Result<()> {
        let show_grand_total = folder_metadata.len() > 1;
        let mut total_files: u64 = 0;
        let mut total_folders: u64 = 0;
        let mut total_size: u64 = 0;
        let mut writer = text::get_writer(&file_option, append)?;
        for metadata in folder_metadata {
            let (files, folders, size) = write_summary_metadata(&mut writer, &metadata)?;
            total_files += files;
            total_folders += folders;
            total_size += size;
        }
        if show_grand_total {
            let grand_total = format!(
                "\n{:>7} {:>7} {:>7} Grand Total",
                to_pretty_size(total_files),
                to_pretty_size(total_folders),
                to_pretty_size(total_size)
            );
            writer.write_all(grand_total.as_bytes())?;
        }
        Ok(())
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
    /// * `writer` - the target for text output.
    /// * `metadata` - the folder metadata that will be summarized.
    fn write_summary_metadata(writer: &mut Box<dyn io::Write>, metadata: &Metadata) -> Result<(u64, u64, u64)> {
        writer.write_all(format!("\n{:>7} {:^7} {:^7} {}", "Files", "Folders", " Size", "Folder Name").as_bytes())?;
        if metadata.is_folder() {
            writer.write_all("\n".as_bytes())?;
            let (files, folders, size) = write_summary_folder(writer, metadata)?;
            let pretty_files = to_pretty_size(files);
            let pretty_folders = to_pretty_size(folders);
            let pretty_size = to_pretty_size(size);
            writeln!(writer, "{:>7} {:>7} {:>7} Total", pretty_files, pretty_folders, pretty_size)?;
            Ok((files, folders, size))
        } else {
            Err(Error::from(format!("'{}' is not a folder!!!", metadata.pathname())))
        }
    }
    /// Produces summary information for a folder.
    /// 
    /// The function recursively calls itself for each subfolder contained by the folder.
    /// A tuple is returned that contains the following summary information
    /// 
    /// * count of files
    /// * count of subfolders
    /// * total disk size used
    /// 
    /// # Arguments
    /// 
    /// * `writer` - the target for text output.
    /// * `metadata` - the folder metadata that will be summarized.
    fn write_summary_folder(writer: &mut Box<dyn io::Write>, metadata: &Metadata) -> Result<(u64, u64, u64)> {
        let mut files: u64 = 0;
        let mut folders: u64 = 0;
        let mut size: u64 = 0;
        if let Metadata::Folder(folder_md) = metadata {
            for metadata in &folder_md.children {
                if metadata.is_folder() {
                    folders += 1;
                } else {
                    // files and symlinks are grouped together
                    files += 1;
                }
                size += metadata.size();
            }
            writer.write_all(
                format!(
                    "{:>7} {:>7} {:>7} {}\n",
                    to_pretty_size(files),
                    to_pretty_size(folders),
                    to_pretty_size(size),
                    metadata.pathname()
                )
                .as_bytes(),
            )?;

            for metadata in &folder_md.children {
                if metadata.is_folder() {
                    let (folder_files, folder_folders, folder_size) = write_summary_folder(writer, metadata)?;
                    files += folder_files;
                    folders += folder_folders;
                    size += folder_size;
                }
            }
        }
        Ok((files, folders, size))
    }

    /// Produces summary information about what filesystem metadata has been loaded.
    /// 
    /// # Arguments
    /// 
    /// * `db_pathname` - the database that will be examined.
    /// * `db_information` - the summary information about the database.
    /// * `file_option` - the optional file where summary information will be written.
    /// * `append` - if output is to a file, append text otherwise overwrite.
    pub fn write_db_information(
        db_pathname: &str,
        db_information: DbInformation,
        file_option: Option<PathBuf>,
        append: bool
    ) -> Result<()> {
        let mut output = format!("Database: {db_pathname}\n");
        writeln!(output, "  Size: {}", to_pretty_size(db_information.database_size))?;
        writeln!(output, "Root Folders:")?;
        for folder in db_information.root_folders {
            writeln!(output, "  {}", folder)?;
        }
        writeln!(output, "Row Counts:")?;
        writeln!(output, "  {:>8}: {:>7}", "Folders", db_information.folder_count.separate_with_commas())?;
        writeln!(output, "  {:>8}: {:>7}", "Files", db_information.file_count.separate_with_commas())?;
        writeln!(output, "  {:>8}: {:>7}", "Problems", db_information.problem_count.separate_with_commas())?;
        let mut writer = text::get_writer(&file_option, append)?;
        write!(writer, "{output}")?;
        Ok(())
    }
    /// Shows what problems were encountered loading filesystem metadata into the database.
    /// 
    /// # Arguments
    /// 
    /// * `problems` - the list of problems that occurred.
    /// * `file_option` - the option file where summary information will be written.
    /// * `append` - if output is to a file, append text otherwise overwrite.
    pub fn write_problems(problems: Vec<Metadata>, file_option: Option<PathBuf>, append: bool) -> Result<()> {
        let mut output = String::default();
        if problems.is_empty() {
            writeln!(output, "There are no problems")?;
        } else {
            for problem in problems {
                if let Metadata::Folder(folder_md) = problem {
                    writeln!(output, "\n{}", folder_md.pathname)?;
                    for child_problem in folder_md.children {
                        if let Metadata::Problem(problem_md) = child_problem {
                            writeln!(output, "    '{}' {}", problem_md.name, problem_md.description)?;
                        }
                    }
                }
            }
        }
        let mut writer = text::get_writer(&file_option, append)?;
        write!(writer, "{output}")?;
        Ok(())
    }

    /// A helper function to determine if a Metadata instance has children.
    #[inline]
    fn has_children(metadata: &Metadata) -> bool {
        match metadata {
            Metadata::Folder(folder_md) => folder_md.children.len() > 0,
            _ => false,
        }
    }

    /// A helper that converts a timestamp to a datetime string.
    /// 
    /// If the timestamp is older than 1 year it will reflect the date otherwise it will inlcude a timestamp.
    fn to_datetime_string(ts: u64) -> String {
        let local_datetime = Local.timestamp(ts as i64, 0);
        if local_datetime.year() != Local::today().year() {
            const OLD_DATETIME_FMT: &str = "%h %_d  %Y";
            format!("{}", local_datetime.format(OLD_DATETIME_FMT))
        } else {
            const CURRENT_DATETIME_FMT: &str = "%h %_d %H:%M";
            format!("{}", local_datetime.format(CURRENT_DATETIME_FMT))
        }
    }
    /// Converts a size to a pretty format.
    /// 
    /// * for values less than 1 Mib output will be formatted ###,###
    /// * for values at least 1 Mib and less than 1 Gib output will be formatted ###.#Mb
    /// * for value at least 1 Gib output will be formatted ###.#Gb
    fn to_pretty_size(size: u64) -> String {
        const MIB: u64 = 1024 * 1024;
        const GIB: u64 = MIB * 1024;
        if size < MIB {
            // ###,###
            size.separate_with_commas()
        } else if size < GIB {
            // ###.#Mb
            let mib: f64 = (size as f64 / MIB as f64) + 0.5;
            format!("{:.1}Mb", mib)
        } else {
            // ###.#Gb
            let gib: f64 = (size as f64 / GIB as f64) + 0.5;
            format!("{:.1}Gb", gib)
        }
    }
}

/// A helper function that creates an absolute pathname from some folder.
fn as_absolute_pathname(folder_name: &str) -> Result<String> {
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
    if cfg!(windows) {
        let mut components = absolute_path.components();
        let component = components
            .next()
            .ok_or(Error::from("Yikes! Path had no components..."))?;
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
