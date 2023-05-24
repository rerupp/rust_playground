//! # The subcommand that reports folders with duplicate files.
//!
//! There are currently three (3) classes of reports that can be created.
//!
//! * List all folders that have a common filename and whether the file contents match. The
//! report list which folders had file matches and those that did not.
//! what other reports will be built from.
//! * List all folders that have matching files and what files matched. It will also indicate if
//! one of the matching folders did not not have matching files.
//! * List all folders that did not have a matching file. It will also indicate if a folder had
//! a file match with another folder.
use super::{
    commafy,
    lib::domain::{
        DuplicateFolders, DuplicateFoldersMatch, FolderAnalysisMd, FolderGroupMd, FoldersMatchMd, FoldersNoMatch,
    },
    mbufmt, rptcols, rptrow,
    text::{get_writer, write_strings, Report},
    FolderMd, PathBuf, Result, Session, StopWatch,
};
use clap::Args;

/// The duplicate files command arguments.
#[derive(Args, Debug)]
pub struct CommandArgs {
    /// Initialize the file duplicates metadata.
    #[clap(long, group = "cmd")]
    init: bool,
    /// Generate a report of all duplicate files and directory groupings.
    #[clap(short, long, group = "cmd")]
    list: bool,
    /// Generate a report of folder groups with duplicate files.
    #[clap(short = 'm', long = "match", group = "cmd")]
    matches: bool,
    /// Generate a report of folders that were not in a match group.
    #[clap(short = 'n', long = "no_match", group = "cmd")]
    none: bool,
    /// Summarize the duplicate files metadata (default).
    #[clap(short, long = "sum", group = "cmd")]
    summary: bool,
    #[clap(
        short = 'r', long = "report", value_name="FILE", forbid_empty_values = true,
        parse(try_from_str = super::parse_filename), requires = "list",
        group = "out"
    )]
    /// The report file pathname.
    pub output_path: Option<PathBuf>,
    /// Append to the report file, otherwise overwrite
    #[clap(short = 'A', long = "append", requires("out"))]
    pub append_log: bool,
}

/// The initialize database command definition.
pub struct Command {
    /// The commands arguments.
    args: CommandArgs,
}
impl Command {
    /// Creates an instance of the command.
    ///
    /// # Arguments
    ///
    /// * `args` are the command arguments that will be used.
    pub fn new(args: CommandArgs) -> Command {
        Command { args }
    }
    /// Manages execution of the various file duplicate sub-commands.
    ///
    /// # Arguments
    ///
    /// * `session` provides the domain API used to implement each command.
    pub fn execute(&self, session: &Session) -> Result<()> {
        let mut report_build = StopWatch::start_new();
        let report = if self.args.init {
            initialize(session)?
        } else if self.args.list {
            let duplicate_folders = session.duplicate_folders_files()?;
            list::report(duplicate_folders)
        } else if self.args.matches {
            let folders_match = session.duplicate_folders_files_match()?;
            matches::report(folders_match)
        } else if self.args.none {
            let folders_no_match = session.duplicate_folders_no_match()?;
            no_matches::report(folders_no_match)
        } else {
            summary(session)?
        };
        report_build.stop();
        log::info!("Report build took {}", report_build);
        let mut writer = get_writer(&self.args.output_path, self.args.append_log)?;
        write_strings(&mut writer, report.into_iter())?;
        Ok(())
    }
}

/// Reloads the duplicate files metadata.
///
/// # Arguments
///
/// * `session` provides the domain API used to implement the command.
fn initialize(session: &Session) -> Result<Report> {
    let mut report = Report::from(rptcols!(<=(2), =));
    report.text(rptrow!(= "Initialize duplicate files"));
    let elapsed = StopWatch::start_new();
    let duplicate_file_count = session.duplicate_files_reload()?;
    log::info!("initialize took {elapsed}");
    report.text(rptrow!(_, format!("{} duplicate files found.", commafy(duplicate_file_count))));
    Ok(report)
}

mod matches {
    //! This module consolidates the implementation of folder file matching report.

    use super::{commafy, mbufmt, rptcols, rptrow, DuplicateFoldersMatch, FoldersMatchMd, Report, StopWatch};

    /// Generates the report showing what folders had matching files.
    ///
    /// # Arguments
    ///
    /// * `folders_match` is the metadata the report will be built from.
    pub fn report(folders_match: DuplicateFoldersMatch) -> Report {
        let overall = StopWatch::start_new();
        let mut report = Report::from(rptcols!(<=(2), <=(2), <=(2), <=(2), =));
        for folder_group in folders_match.into_iter() {
            folder_group_report(&mut report, &folder_group);
        }
        log::info!("match report elapsed: {}", overall);
        report
    }

    /// Generates the report content for a group of folders that have matching files.
    ///
    /// # Arguments
    ///
    /// * `report` is updated with information about the matching folder group.
    /// * `folders_match` is the metadata about the matching folders.
    fn folder_group_report(report: &mut Report, folders_match: &FoldersMatchMd) {
        report.text(rptrow!(="Folders filename match"));
        report.text(rptrow!(_, ="Pathnames:"));
        folders_match.folders_md.iter().for_each(|&md| {
            report.text(rptrow!(_, _, = &md.pathname, =format!("({} files)", commafy(md.children.len()))));
        });
        report.text(rptrow!(_, ="Common filenames:", =format!("({} files)", commafy(folders_match.matches.len()))));
        report.text(rptrow!(_, _, =folders_match.matches.join(", ")));
        let (actual, total) = files_size(folders_match);
        report.text(rptrow!(_, _, = format!(
            "Total: {} Actual: {} Recoverable: {}", mbufmt!(total), mbufmt!(actual), mbufmt!(total - actual)
        )));
        if folders_match.except.len() > 0 {
            report.text(rptrow!(_, ="Files that did not match:"));
            report.text(rptrow!(_, _, =folders_match.except.join(", ")));
        }
        if folders_match.other_matches.len() > 0 {
            report.text(rptrow!(_, ="Folders that have other matches"));
            for (folder_md, folder_groups) in &folders_match.other_matches {
                report.text(rptrow!(_, _, =format!("{}:", folder_md.pathname)));
                for folders_group_md in folder_groups {
                    report.text(rptrow!(_, _, _, ="Folder group:"));
                    folders_group_md.iter().for_each(|md| {
                        report.text(rptrow!(_, _, _, _, = &md.pathname));
                    });
                }
            }
        }
    }

    /// Calculates the size of matching files along with the actual disk usage.
    ///
    /// # Arguments
    ///
    /// * `folders_match` is the metadata about the matching folders.
    fn files_size(folders_match: &FoldersMatchMd) -> (u64, u64) {
        // any folders md will do
        let &folder_md = &folders_match.folders_md[0];
        let actual: u64 = folders_match.matches.iter().map(|&filename| folder_md.children[filename].size()).sum();
        (actual, actual * folders_match.folders_md.len() as u64)
    }
}

mod no_matches {
    //! This module consolidates the implementation of folders that did not have matching files report.

    use super::{commafy, rptcols, rptrow, FoldersNoMatch, Report};

    /// Generate the report showing folders that did not have matches.
    ///
    /// # Arguments
    ///
    /// * `folders_no_match` is the metadata about folders that did not match.
    pub fn report(folders_no_match: FoldersNoMatch) -> Report {
        let mut report = Report::from(rptcols!(<=(2), <=(2), <=(2), =));
        report.text(rptrow!(="Folders Without Matches"));
        for no_match in folders_no_match.into_iter() {
            let children_len = no_match.folder_md.children.len();
            let filenames_len = no_match.filenames.len();
            let descr = match children_len == filenames_len {
                true => format!("(All {} files)", commafy(children_len)),
                false => format!("({} files)", commafy(children_len)),
            };
            report.text(rptrow!(_, = &no_match.folder_md.pathname, =descr));
            if children_len != filenames_len {
                report.text(rptrow!(_, _, ="Filenames:", =format!("({} files)", commafy(filenames_len))));
                report.text(rptrow!(_, _, _, no_match.filenames.join(", ")));
                let label = match no_match.other_matches == 1 {
                    true => "file",
                    false => "files",
                };
                report.text(rptrow!(_, _, =format!("{} {label} did match.", commafy(no_match.other_matches))));
            }
        }
        report
    }
}

mod list {
    //! This module consolidates the report showing all the folders with common filenames and if they
    //! matched or not.

    use super::{
        commafy, mbufmt, rptcols, rptrow, DuplicateFolders, FolderAnalysisMd, FolderGroupMd, FolderMd, Report,
        StopWatch,
    };

    /// Create a report of the duplicate filenames.
    ///
    /// # Arguments
    ///
    /// * `duplicate_folders` is the duplicate filename metadata.
    pub fn report(duplicate_folders: DuplicateFolders) -> Report {
        let mut size: u64 = 0;
        let mut used: u64 = 0;
        let mut report = Report::from(rptcols!(<=(2), <=(2), <=(2), =));
        let report_build = StopWatch::start_new();
        for folders_group in duplicate_folders.into_iter() {
            let (folder_size, folder_used) = folders_group_report(&mut report, &folders_group);
            size += folder_size;
            used += folder_used;
        }
        report.text(rptrow!(= format!(
            "Total disk space: actual {}, used {}, recoverable {}", mbufmt!(size), mbufmt!(used), mbufmt!(used - size)
        )));
        log::info!("list took {} to build.", report_build);
        report
    }

    /// Generate a report section for a group of folders with common filenames.
    /// 
    /// The tuple returned contains the size of files and disk space being used respectively.
    /// 
    /// # Arguments
    /// 
    /// * `report` will be updated with the matching and non-matching folders and filenames.
    /// * `folder_group` is the metadata describing the match results.
    fn folders_group_report(report: &mut Report, folder_group: &FolderGroupMd) -> (u64, u64) {
        report.text(rptrow!(= "Folders Group"));
        // collect up name and size from the metadata
        let folder_info: Vec<(&str, u64)> = folder_group
            .folders_md
            .iter()
            .map(|&folder_md| (folder_md.pathname.as_str(), folder_md.children.len() as u64))
            .collect();
        folder_info.iter().for_each(|&(folder_name, children)| {
            report.text(rptrow!(_, = folder_name, = format!("({} files)", commafy(children))));
        });
        // show the duplicate filenames
        report.text(rptrow!(_, = "Group filenames:"));
        report.text(rptrow!(_, _, = folder_group.filenames.join(", ")));
        let (size, used) = folder_matches_report(report, &folder_group.folder_analysis);
        folder_no_match_report(report, &folder_group.folder_analysis);
        (size, used)
    }

    /// Generate the report section describing folders with matching files.
    ///
    /// The tuple returned contains the size of files and disk space being used respectively.
    /// 
    /// # Arguments
    /// 
    /// * `report` will be updated with the folders and file matches.
    /// * `folder_analysis` is the metadata that describes the match results.
    fn folder_matches_report(report: &mut Report, folder_analysis: &FolderAnalysisMd) -> (u64, u64) {
        // show the folders with matching files
        report.text(rptrow!(_, = "Folder Matches"));
        let mut size: u64 = 0;
        let mut used: u64 = 0;
        if folder_analysis.file_matches.is_empty() {
            report.text(rptrow!(_, _, = "None"));
        } else {
            for (folders_md, filenames) in &folder_analysis.file_matches {
                // let (folder_size, folders_used) = folders_md_report(report, folders_md.clone(), filenames.clone());
                let (folder_size, folders_used) = folders_md_report(report, folders_md, filenames);
                size += folder_size;
                used += folders_used;
            }
            report.text(rptrow!(_, _, _, = format!(
                "Folder disk space: actual {}, used {}, recoverable {}",
                mbufmt!(size),
                mbufmt!(used),
                mbufmt!(used - size))));
        }
        (size, used)
    }

    /// Create the report section describing folders with file matches.
    /// 
    /// The tuple returned contains the size of files and disk space being used respectively.
    /// 
    /// # Arguments
    /// 
    /// * `report` will be updated with details about the folders that have matching files.
    /// * `folders_md` contains the folders metadata.
    /// * `filenames` is the list of files that matched.
    fn folders_md_report(report: &mut Report, folders_md: &Vec<&FolderMd>, filenames: &Vec<&str>) -> (u64, u64) {
        // log::trace!("folder group");
        // TODO: the FolderMd is now sorted by pathname so fix this duplication
        let mut folder_names: Vec<&str> = folders_md.iter().map(|folder_md| folder_md.pathname.as_str()).collect();
        folder_names.sort();
        folder_names.into_iter().for_each(|folder_name| {
            // log::trace!("  {}", folder_name);
            report.text(rptrow!(_, _, = folder_name));
        });
        // any folders metadata will do
        let &folder_md = &folders_md[0];
        let mut size: u64 = 0;
        let mut names: Vec<&str> = vec![];
        filenames.iter().for_each(|&filename| {
            size += folder_md.children[filename].size();
            names.push(filename);
        });
        let used = size * folders_md.len() as u64;
        // show the matching filenames for the group
        // TODO: the names are already sorted so fix this duplication
        names.sort();
        report.text(rptrow!(_, _, _, names.join(", ")));
        // log::trace!("  size {} used {}", mbufmt!(size), mbufmt!(used));
        (size, used)
    }

    /// Generate the report section describing folders without matching files.
    /// 
    /// # Arguments
    /// 
    /// * `report` will be updated with the folders and files that did not match.
    /// * `folder_analysis` is the metadata that describes the match results.
    fn folder_no_match_report(report: &mut Report, folder_analysis: &FolderAnalysisMd) {
        report.text(rptrow!(_, = "Folders With No Match"));
        if folder_analysis.files_without_match.is_empty() {
            report.text(rptrow!(_, _, = "None"));
        } else {
            folder_analysis.files_without_match.iter().for_each(|(folder_md, _)| {
                report.text(rptrow!(_, _, = &folder_md.pathname));
            });
        }
    }
}

///  List a summary of the duplicate files metadata.
///
/// # Arguments
///
/// *`session` the `domain` session that will be used to reload metadata.
fn summary(session: &Session) -> Result<Report> {
    let (folder_cnt, file_cnt) = session.duplicate_files_summary()?;
    let mut report = Report::from(rptcols!(<=(2), <));
    report.header(rptrow!(= "Duplicate files summary"));
    report.text(rptrow!(_, format!("Duplicate filenames: {}", commafy(file_cnt))));
    report.text(rptrow!(_, format!("Folders with duplicate filenames: {}", commafy(folder_cnt))));
    Ok(report)
}
