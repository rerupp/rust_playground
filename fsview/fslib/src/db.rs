//! # The filesystem metadata persistence module.
//! 
//! This module contains the API to initialize, load, and query metadata in the
//! database. It is currently uses SQLite3 as the database engine. The database
//! is self contained and there is no need to install other software (unless you
//! want the SQLite3 tools). The SQL for schema initialization and query is stored
//! in a `sql` directory within the module.
mod load;
mod query;
mod schema;

use super::{domain, filesys, StopWatch};

use rusqlite as sql;
use std::{fmt, path::PathBuf, result};

#[rustfmt::skip]
pub(crate) use {
    load::{
        load_fs_metadata,
        file_duplicates_reload,
    },
    query::{
        database_metrics_query,
        duplicate_ids,
        // duplicate_filename_metadata_query,
        duplicate_files_metadata_query,
        duplicate_file_metrics,
        folder_content_by_name_query,
        folder_content_by_pathname_query,
        folder_tree_by_name_query,
        folder_tree_by_pathname_query,
        get_table_counts_query,
        problems_query,
        root_folder_content_query,
        root_folders_pathname_query,
    },
    schema::{
        drop as schema_drop,
        init as schema_init,
    },
};

/// The result of calling a function in this module.
type Result<T> = result::Result<T, Error>;

/// The type of error returned from the module.
///
/// I like the pattern of a module consolidating errors. In this case
/// collecting the `io::Error` and mapping it to the module error hides
/// details concerning the implementation.
#[derive(fmt::Debug)]
pub struct Error(String);
/// Include the [`ToString`] trait for the domain [`Error`].
impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// Create an error from a string.
impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(format!("db: {error}"))
    }
}
/// Convert a SQL error to an error.
impl From<sql::Error> for Error {
    fn from(error: sql::Error) -> Self {
        Error(format!("sql: {error}"))
    }
}


/// Create a connection to the database.
/// 
/// # Arguments
/// 
/// * `db_path_option` - a path to the database, if `None` then an in-memory database will be used.
pub(crate) fn database_connection(db_path_option: Option<&PathBuf>) -> Result<sql::Connection> {
    let conn = match db_path_option {
        Some(db_path) => sql::Connection::open(db_path)?,
        None => sql::Connection::open_in_memory()?,
    };
    Ok(conn)
}

/// The filename for a dummy file used when a folder does not have any files.
pub const EMPTY_FOLDER_FILENAME: &str = r"<?>";

/// The parent identifier for a folder that was loaded from the filesystem.
pub const ROOT_FOLDER_PARENT_ID: i64 = 0;
