//! Filesystem metadata miner.
//!
//! The module collects metadata for a filesystem folder hiearchy. The module defines a collection
//! of objects representing the collected metadata.
//! * [FolderMetadata]
//! * [FileMetadata]
//! * [ProblemMetadata]
//! * [FsMetadata]
//!
//! The domain calls [collect_metadata] to collect metadata for that folder.
use std::{
    ffi::OsString,
    fmt,
    fs::{self, DirEntry, Metadata},
    io,
    path::{Path, PathBuf},
    result,
    time::SystemTime,
};

use super::StopWatch;
use serde::{Deserialize, Serialize};

/// The result of calling a function in this module.
type Result<T> = result::Result<T, Error>;

/// The type of error returned from the module.
///
/// I like the pattern of a module consolidating errors. In this case
/// collecting the `io::Error` and mapping it to the module error hides
/// details concerning the implementation.
///
#[derive(fmt::Debug)]
pub struct Error(
    /// A description of what happened.
    ///
    /// The use case for errors so far is when one happens show a message
    /// describing the details and stop. The need to have finer grained
    /// error handling is not there.
    String,
);

/// Fullfill the requirement of an error
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Create an error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(format!("filesys: {error}"))
    }
}

/// Convert an IO error to the module error.
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error(format!("io: {error}"))
    }
}

/// Catch a bad Unicode error converting to a string.
impl From<OsString> for Error {
    fn from(error: OsString) -> Self {
        Error(format!("Invalid Unicode: '{}'", error.to_string_lossy()))
    }
}

/// Convert the error to a string.
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.0
    }
}

/// The metadata associated with a folder.
#[derive(fmt::Debug, PartialEq, Serialize, Deserialize)]
pub struct FolderMetadata {
    /// The folder pathname.
    pub path: PathBuf,
    /// The folder disk space being used.
    pub size: u64,
    /// A timestamp of when the folder was created or 0 if not available.
    pub created: u64,
    /// A timestamp of when the folder was last modified or 0 if not available.
    pub modified: u64,
    /// The contents of the folder.
    pub children: Vec<FsMetadata>,
}

impl FolderMetadata {
    /// Create an instance of the folder metadata.
    ///
    /// # Arguments
    /// * `folder_path` - The folder name in the file system.
    /// * `metadata` - The filesystem metadata for the folder.
    ///
    /// # Note
    ///
    /// The created and modified timestamp may not be available for whatever reason. If
    /// it is not available the timestamp will be set to 0.
    ///
    pub fn new(folder_path: &PathBuf, metadata: Metadata) -> Self {
        FolderMetadata {
            path: folder_path.clone(),
            size: metadata.len(),
            created: metadata.created().map_or(0, |system_time| file_timestamp(system_time)),
            modified: metadata.modified().map_or(0, |system_time| file_timestamp(system_time)),
            children: vec![],
        }
    }
    /// Returns the folder pathname.
    pub fn pathname(&self) -> String {
        as_pathname(&self.path)
    }
    /// Returns the folder filename.
    pub fn filename(&self) -> String {
        as_filename(&self.path)
    }
}

/// The metadata associated with a file.
#[derive(fmt::Debug, PartialEq, Serialize, Deserialize)]
pub struct FileMetadata {
    /// The file pathname.
    pub path: PathBuf,
    /// Indicates the file is a link to another filesystem item.
    pub is_symlink: bool,
    /// The file disk space being used.
    pub size: u64,
    /// A timestamp of when the file was created or 0 if not available.
    pub created: u64,
    /// A timestamp of when the file was last modified or 0 if not available.
    pub modified: u64,
}

impl FileMetadata {
    /// Create an instance of the file metadata.
    ///
    /// # Arguments
    /// * `dir_entry` - The metadata of an item contained in a folder within the file system.
    ///
    /// # Note
    ///
    /// The created and modified timestamp may not be available for whatever reason. If
    /// it is not available the timestamp will be set to 0.
    ///
    fn new(dir_entry: &DirEntry) -> Result<FileMetadata> {
        let pathname = dir_entry.path();
        let metadata = dir_entry.metadata()?;
        Ok(FileMetadata {
            path: pathname,
            is_symlink: metadata.file_type().is_symlink(),
            size: metadata.len(),
            created: metadata.created().map_or(0, |system_time| file_timestamp(system_time)),
            modified: metadata.modified().map_or(0, |system_time| file_timestamp(system_time)),
        })
    }
    /// Returns the file pathname.
    pub fn pathname(&self) -> String {
        as_pathname(&self.path)
    }
    /// Returns the file filename.
    pub fn filename(&self) -> String {
        as_filename(&self.path)
    }
}

/// The metadata for some problem, such as *access denied*, when trying to get
/// metadata for a folder or file.
#[derive(fmt::Debug, PartialEq, Serialize, Deserialize)]
pub struct ProblemMetadata {
    /// The pathname of the folder or file.
    path: PathBuf,
    /// A description of what happened.
    pub description: String,
}

impl ProblemMetadata {
    /// Creates a new instance of the problem metadata.
    ///
    /// # Arguments
    ///
    /// * `path` - the pathname of the folder or file.
    /// * `description` - a description of what happened.
    fn new(path: &PathBuf, description: String) -> ProblemMetadata {
        ProblemMetadata {
            path: path.clone(),
            description,
        }
    }
    /// Returns the folder or file pathname.
    pub fn pathname(&self) -> String {
        as_pathname(&self.path)
    }
}

/// The type of filesystem metadata and the possible variants.
#[derive(fmt::Debug, PartialEq, Serialize, Deserialize)]
pub enum FsMetadata {
    /// Contains the metadata for a folder.
    Folder(FolderMetadata),
    /// Contains the metadata for a file.
    File(FileMetadata),
    /// Contains the metadata of some problem that occurred.
    Problem(ProblemMetadata),
}

impl FsMetadata {
    /// Get the metadata pathname.
    pub fn path(&self) -> &Path {
        match self {
            FsMetadata::Folder(folder_metadata) => &folder_metadata.path,
            FsMetadata::File(file_metadata) => &file_metadata.path,
            FsMetadata::Problem(problem_metadata) => &problem_metadata.path,
        }
    }
    /// Identifies the metadata to be the `File` variant.
    pub fn is_file(&self) -> bool {
        match self {
            FsMetadata::File(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for FsMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Folder(folder_md) => write!(f, "Folder({})", folder_md.pathname()),
            Self::File(file_md) => write!(f, "File({})", file_md.pathname()),
            Self::Problem(problem_md) => write!(f, "Problem({})", problem_md.pathname()),
        }
    }
}

/// Collect the metadata for a folder hierarchy.
///
/// # Arguments
/// * `folder_path` - the path to some folder.
///
/// # Note
/// An error will be returned if the `folder_path` does not exist or if it is not a folder.
pub fn collect_metadata(folder_path: &PathBuf) -> Result<FsMetadata> {
    if folder_path.exists() {
        let folder_path = if cfg!(windows) {
            // the windows version of canonicalize comes back as a Win32 file I/O namesapce (\\?\drive:\directory_path)
            // and this removes the "\\?\" prefix
            let win32_path = std::fs::canonicalize(folder_path.clone())?;
            let win32_string = win32_path.into_os_string().into_string()?;
            PathBuf::from(&win32_string[4..])
        } else {
            std::fs::canonicalize(folder_path.clone())?
        };
        let collect_time = StopWatch::start_new();
        let folder = visit_folder(&folder_path)?;
        log::debug!("collect_metadata={collect_time}");
        if log::log_enabled!(log::Level::Trace) {
            dump_metadata(&folder);
        }
        Ok(folder)
    } else {
        Err(Error::from(format!("{} does not exist...", folder_path.as_path().display())))
    }
}

/// A function that walks the filesystem metadata and logs its contents. `TRACE` level logging must be
/// in effect in order for the metadata to be logged.
fn dump_metadata(metadata: &FsMetadata) {
    match metadata {
        FsMetadata::Problem(problem_md) => log::trace!("Problem: {}", problem_md.pathname()),
        FsMetadata::File(file_md) => {
            let file_type = if file_md.is_symlink { "Symlink" } else { "File" };
            log::trace!("{file_type}: {}", file_md.pathname())
        }
        FsMetadata::Folder(folder_md) => {
            log::trace!("Folder: {}", folder_md.pathname());
            for child_metadata in &folder_md.children {
                dump_metadata(child_metadata);
            }
        }
    }
}

/// Retrieves the metadata for a folder.
///
/// This function will call itself recursively for each child folder. It guarantees the
/// child metadata is ordered by its name.
fn visit_folder(folder_path: &PathBuf) -> Result<FsMetadata> {
    if !folder_path.is_dir() {
        Err(Error::from(format!("files: {} is not a folder!!!", folder_path.display())))
    } else {
        let fs_metadata: FsMetadata = match fs::read_dir(folder_path) {
            // this prevents 'access denied' from blowing up the traversal
            Err(error) => FsMetadata::Problem(ProblemMetadata::new(folder_path, format!("{error}"))),
            Ok(read_dir) => {
                let mut folder_metadata = FolderMetadata::new(folder_path, folder_path.metadata()?);
                for read_result in read_dir {
                    match read_result {
                        // not sure yet how to get here, maybe a Linux thing where the directory
                        // can be read but a file in the directory cannot be stat'd??? or maybe a
                        // filesystem error??? regardless don't treat it the same the fs::read_dir error
                        Err(error) => {
                            log::error!("Error reading DirEntry! {error}");
                            let problem_md = ProblemMetadata::new(folder_path, format!("{error}"));
                            folder_metadata.children.push(FsMetadata::Problem(problem_md));
                        }
                        Ok(dir_entry) => {
                            let entry_path = dir_entry.path();
                            let fs_node = if entry_path.is_dir() {
                                visit_folder(&entry_path)?
                            } else {
                                FsMetadata::File(FileMetadata::new(&dir_entry)?)
                            };
                            folder_metadata.children.push(fs_node);
                        }
                    }
                }
                folder_metadata.children.sort_by(|lhs, rhs| lhs.path().cmp(&rhs.path()));
                FsMetadata::Folder(folder_metadata)
            }
        };
        Ok(fs_metadata)
    }
}

/// Converts a filesystem timesamp into the number of seconds since the [SystemTime::UNIX_EPOCH].
fn file_timestamp(system_time: SystemTime) -> u64 {
    match system_time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
/// Convert a filesystem path to a string.
fn as_pathname(path: &Path) -> String {
    path.display().to_string()
}
/// Retrieves the filename of a filesystem path.
fn as_filename(path: &Path) -> String {
    path.file_name().map_or("", |f| f.to_str().unwrap_or("")).to_string()
}
/// Count the items contained by the filesystem metadata.
pub fn count_metadata(fs_metadata: &FsMetadata) -> i64 {
    let mut count: i64 = 0;
    let mut counter = || count += 1;
    fn traverse<F: FnMut()>(fs_node: &FsMetadata, f: &mut F) {
        f();
        if let FsMetadata::Folder(folder_metadata) = fs_node {
            for child in &folder_metadata.children {
                traverse(&child, f);
            }
        }
    }
    traverse(fs_metadata, &mut counter);
    count
}
