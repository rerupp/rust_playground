//! # Domain objects used by cli and files modules
//!
use rusqlite as sql;
use std::{fmt, path::PathBuf, result};

use super::{db, filesys};

mod api;
mod builders;
mod objects;

use builders::Hierarchy as HierarchyBuilder;
pub use objects::DbInformation;
pub use objects::FileMd;
pub use objects::FolderMd;
pub use objects::Metadata;
pub use objects::ProblemMd;

/// The type of result returned from the domain.
pub type Result<T> = result::Result<T, Error>;

/// The type of error returned from the module.
///
/// I like the pattern of a module consolidating errors. In this case
/// collecting the `fmt::Error`, `db::Error`, `filesys::Error`, etc. and
/// mapping it to the module error hides details concerning the implementation.
#[derive(fmt::Debug)]
pub struct Error(pub String);
/// Fullfill the requirement of an error
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// Create an error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(format!("domain: {error}"))
    }
}
/// Convert a `fmt::Error` to a domain error.
impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error(format!("fmt: {error}"))
    }
}
/// Convert a `db::Error` to a domain error.
impl From<db::Error> for Error {
    fn from(error: db::Error) -> Self {
        // simply remap the error
        Error(error.0)
    }
}
/// Convert a `filesys::Error` to a domain error.
impl From<filesys::Error> for Error {
    fn from(error: filesys::Error) -> Self {
        // simply remap the error
        Error(String::from(error))
    }
}
/// Get an instance of the `domain` API.
/// 
/// If the path option is `None` the cargo package name will be used as the
/// database name with an extension of *db*.
pub fn get_session(path_option: Option<PathBuf>) -> Result<Session> {
    let db_path = if let Some(path) = path_option {
        path
    } else {
        let package_name = option_env!("CARGO_PKG_NAME");
        PathBuf::from(package_name.unwrap_or("fsview")).with_extension("db")
    };
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
/// Allow the session to be used by `format!` to display it's contents.
impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.db_path.as_path().display())
    }
}

impl Session {
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
}
