//! # Domain objects used by cli and files modules
//!
use rusqlite as sql;
use std::{fmt, path::PathBuf, result};

use super::{db, filesys, StopWatch};

mod api;
mod filedups;
mod objects;

pub(crate) use filedups::DuplicateFoldersBuilder;
pub use filedups::{
    DuplicateFolders, DuplicateFoldersMatch, FolderAnalysisMd, FolderGroupId, FolderGroupMd, FolderNoMatchMd,
    FoldersMatchMd, FoldersNoMatch,
};
pub(crate) use objects::DuplicateIds;
pub use objects::{DbInformation, FileMd, FolderMd, Metadata, ProblemMd};

/// The type of result returned from the domain.
pub type Result<T> = result::Result<T, Error>;

/// The type of error returned from the module.
///
/// I like the pattern of a module consolidating errors. In this case
/// collecting the `fmt::Error`, `db::Error`, `filesys::Error`, etc. and
/// mapping it to the module error hides details concerning the implementation.
#[derive(Debug)]
pub struct Error(String);
/// Include the [`ToString`] trait for the domain [`Error`].
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// This is the domain error handler.
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(format!("domain: {error}"))
    }
}
/// Create a domain error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::from(error.as_str())
    }
}
/// Convert a `db::Error` to a domain error.
impl From<db::Error> for Error {
    fn from(error: db::Error) -> Self {
        // simply remap the error
        Error(error.to_string())
    }
}
/// Convert a `filesys::Error` to a domain error.
impl From<filesys::Error> for Error {
    fn from(error: filesys::Error) -> Self {
        // simply remap the error
        Error(error.to_string())
    }
}

/// Get an instance of the `domain` API.
///
/// # Arguments
/// 
/// * `db_path` is the database that will be used by the session.
pub fn get_session(db_path: PathBuf) -> Result<Session> {
    log::trace!("Session({})", db_path.as_path().display());
    let conn = db::database_connection(Some(&db_path))?;
    Ok(Session { db_path, conn })
}
/// The `domain` session.
#[derive(Debug)]
pub struct Session {
    /// The name of the database.
    db_path: PathBuf,
    /// A SQL connection to the database.
    conn: sql::Connection,
}
impl Session {
    /// Get the name of the database being used.
    pub fn db(&self) -> String {
        self.db_path.as_path().display().to_string()
    }
    /// Add a folder hierarchy to the database.
    ///
    /// # Arguments
    /// * `folder_pathname` - a filesystem folder whose hierarchy will be added to the database.
    pub fn add_folder(&self, folder_pathname: &PathBuf) -> Result<()> {
        if folder_pathname.is_dir() {
            // don't require a mutable session in order to pass in a mutable connection to the api
            let load_conn = db::database_connection(Some(&self.db_path))?;
            api::add_filesystem_folder(load_conn, folder_pathname)
        } else {
            Err(Error(format!("{} must be a filesystem folder!!!", folder_pathname.as_path().display())))
        }
    }
    /// Initialize the database schema.
    ///
    /// # Arguments
    /// * `drop_database` - if `true` the database will be dropped before applying the schema otherwise
    /// changes to the schema will be applied to an existing database.
    pub fn initialize_db(&self, drop_database: bool) -> Result<()> {
        api::initialize_db(&self.conn, drop_database)
    }
    /// Get database metrics and properties.
    pub fn get_db_information(&self) -> Result<DbInformation> {
        api::get_db_information(&self.conn)
    }
    /// Get folder metadata by the folder filename.
    ///
    /// # Arguments
    /// * `folder_name` - the folder name to search for.
    /// * `recursive` - if `true` the folder hierarchy will be returned otherwise just the content
    /// of the folder.
    pub fn get_folder_by_name(&self, folder_name: &str, recursive: bool) -> Result<Vec<Metadata>> {
        api::get_folder_by_name(&self.conn, folder_name, recursive)
    }
    /// Get folder metadata by a folders pathname.
    ///
    /// # Arguments
    /// * `folder_pathname` - the folder path to search for.
    /// * `recursive` - if `true` the folder hierarchy will be returned otherwise just the content
    /// of the folder.
    pub fn get_folder_by_pathname(&self, folder_pathname: &str, recursive: bool) -> Result<Vec<Metadata>> {
        api::get_folder_by_pathname(&self.conn, folder_pathname, recursive)
    }
    /// Get the problems that were encountered adding folders to the database.
    pub fn get_problems(&self) -> Result<Vec<Metadata>> {
        api::get_problems(&self.conn)
    }
    /// Get the contents of the top level folder added to the database.
    pub fn get_root_content(&self) -> Result<Vec<Metadata>> {
        api::get_root_content(&self.conn)
    }
    /// Loads the duplicate files table.
    pub fn duplicate_files_reload(&self) -> Result<u64> {
        api::file_duplicates_reload(&self.conn)
    }
    /// Loads the duplicate files table.
    pub fn duplicate_files_summary(&self) -> Result<(u64, u64)> {
        api::file_duplicates_summary(&self.conn)
    }
    /// Get the metadata concerning all duplicate folders and files.
    pub fn duplicate_folders_files(&self) -> Result<DuplicateFolders> {
        api::duplicate_folders_metadata(&self.conn)
    }
    /// Get the metadata for folders that have duplicate file contents.
    pub fn duplicate_folders_files_match(&self) -> Result<DuplicateFoldersMatch> {
        api::folders_match_metadata(&self.conn)
    }
    /// Get the metadata for folders file content that did not match other folders file content.
    pub fn duplicate_folders_no_match(&self) -> Result<FoldersNoMatch> {
        api::folders_no_match_metadata(&self.conn)
    }
}
