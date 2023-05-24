//! The API that loads a database with filesystem metadata.
use super::{
    filesys::{FileMetadata, FolderMetadata, FsMetadata, ProblemMetadata},
    sql, Error, Result, StopWatch,
};

use std::{fmt, ops};

/// The main function API called from the `domain` to load filesystem metadata.
///
/// # Arguments
///
/// * `conn` - the database connection.
/// * `fs_metadata` - the filesystem metadata that will be added to the database.
pub(crate) fn load_fs_metadata(conn: &mut sql::Connection, fs_metadata: &FsMetadata) -> Result<()> {
    let transaction = conn.transaction()?;
    let mut timer = StopWatch::start_new();
    let insert_count = insert_fs_metadata(&transaction, fs_metadata, super::ROOT_FOLDER_PARENT_ID)?;
    log::debug!("insert={timer}");
    timer.start();
    transaction.commit()?;
    log::debug!("commit={timer}");
    log::info!("{insert_count}");
    Ok(())
}

/// The primary internal API to insert filesystem metadata.
///
/// This function will be called recursively to process the filesystem metadata.
///
/// # Arguments
///
/// * `tx` - the database transaction used to insert metadata.
/// * `fs_metadata` - the filesystem metadata to insert.
/// * `parent_id` - the parent id for data being inserted.
fn insert_fs_metadata(tx: &sql::Transaction, fs_metadata: &FsMetadata, parent_id: i64) -> Result<InsertCount> {
    match fs_metadata {
        FsMetadata::File(file_md) => insert_files(tx, file_md, parent_id),
        FsMetadata::Folder(folder_md) => insert_folders(tx, folder_md, parent_id),
        FsMetadata::Problem(problem_md) => insert_problems(tx, problem_md, parent_id),
    }
}

/// The SQL used to insert folder metadata.
pub const FOLDERS_INSERT: &str = r#"
    INSERT INTO folders
    (parent_id, pathname, name, size, created, modified)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
"#;

/// The internal API that inserts folder metadata.
///
/// # Arguments
///
/// * `tx` - the database tranasction used to insert folder metadata.
/// * `folder_md` - the folder metadata.
/// * `parent_id` - the folder parent identifier.
fn insert_folders(tx: &sql::Transaction, folder_md: &FolderMetadata, parent_id: i64) -> Result<InsertCount> {
    let params =
        (parent_id, folder_md.pathname(), folder_md.filename(), folder_md.size, folder_md.created, folder_md.modified);
    match tx.execute(FOLDERS_INSERT, params) {
        Err(error) => Err(Error::from(format!("directory='{}' {error}.", folder_md.pathname()))),
        _ => {
            log::trace!("FOLDER: {}", folder_md.pathname());
            let mut insert_count = InsertCount::default() + ADD_FOLDER;
            let parent_id = tx.last_insert_rowid();
            let mut has_file = false;
            for child in &folder_md.children {
                if child.is_file() {
                    has_file = true;
                }
                insert_count += insert_fs_metadata(tx, child, parent_id)?;
            }
            if !has_file {
                let fileless_folder = empty_folder_file(&folder_md.pathname());
                insert_fs_metadata(tx, &FsMetadata::File(fileless_folder), parent_id)?;
                insert_count.empty_files += 1;
            }
            Ok(insert_count)
        }
    }
}

/// The SQL used to insert file metadata.
pub const FILES_INSERT: &str = r#"
    INSERT INTO files
    (parent_id, pathname, name, is_symlink, size, created, modified)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#;

/// The internal API that inserts file metadata.
///
/// # Arguments
///
/// * `tx` - the database tranasction used to insert folder metadata.
/// * `file_md` - the file metadata.
/// * `parent_id` - the file parent identifier.
fn insert_files(tx: &sql::Transaction, file_md: &FileMetadata, parent_id: i64) -> Result<InsertCount> {
    log::trace!(
        "{}: {}",
        if file_md.is_symlink {
            "SYMLINK"
        } else if file_md.filename() == super::EMPTY_FOLDER_FILENAME {
            "EFF"
        } else {
            "FILE"
        },
        file_md.pathname(),
    );
    let params = (
        parent_id,
        file_md.pathname(),
        file_md.filename(),
        file_md.is_symlink,
        file_md.size,
        file_md.created,
        file_md.modified,
    );
    match tx.execute(FILES_INSERT, params) {
        Err(error) => Err(Error::from(format!("file='{}' {error}.", file_md.pathname()))),
        _ => Ok(ADD_FILE),
    }
}

/// The SQL used to insert problem metadata.
pub const PROBLEMS_INSERT: &str = r#"
    INSERT INTO problems
    (parent_id, pathname, description)
    VALUES (?1, ?2, ?3)
"#;

/// The internal API that inserts problem metadata.
///
/// # Arguments
///
/// * `tx` - the database tranasction used to insert folder metadata.
/// * `problem_md` - the problem metadata.
/// * `parent_id` - the problem parent identifier.
fn insert_problems(tx: &sql::Transaction, problem_md: &ProblemMetadata, parent_id: i64) -> Result<InsertCount> {
    log::trace!("PROBLEM: {}", problem_md.pathname());
    let params = (parent_id, problem_md.pathname(), problem_md.description.clone());
    match tx.execute(PROBLEMS_INSERT, params) {
        Err(error) => Err(Error::from(format!("db: problem='{}' {error}.", problem_md.pathname()))),
        _ => Ok(ADD_PROBLEM),
    }
}

/// Special case a folder that does not contain a file. Add a ficticious row so the folder will show up when
/// folders and files are joined
fn empty_folder_file(folder_pathname: &str) -> FileMetadata {
    let mut file_path = super::PathBuf::from(folder_pathname);
    file_path.push(super::EMPTY_FOLDER_FILENAME);
    FileMetadata { path: file_path, is_symlink: false, size: 0, created: 0, modified: 0 }
}

/// An internal structure that tracks insert counts into the database.
#[derive(Debug, Default)]
struct InsertCount {
    /// The count of folders inserted.
    folders: usize,
    /// The count of files inserted.
    files: usize,
    /// The count of problems inserted.
    problems: usize,
    /// The count of empty folder files inserted.
    empty_files: usize,
}

/// Used when a folder is added to the database to update the [InsertCount].
const ADD_FOLDER: InsertCount = InsertCount { folders: 1, files: 0, problems: 0, empty_files: 0 };

/// Used when a file is added to the database to update the [InsertCount].
const ADD_FILE: InsertCount = InsertCount { folders: 0, files: 1, problems: 0, empty_files: 0 };

/// Used when a file is added to the database to update the [InsertCount].
const ADD_PROBLEM: InsertCount = InsertCount { folders: 0, files: 0, problems: 1, empty_files: 0 };

/// Create a nicely formated output of the insert count fields.
impl fmt::Display for InsertCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Insert Counts")?;
        writeln!(f, "  folders: {}", self.folders)?;
        writeln!(f, "  files: {}", self.files)?;
        writeln!(f, "  problems: {}", self.problems)?;
        writeln!(f, "  entries: {}", self.folders + self.files + self.problems)?;
        writeln!(f, "  empty files: {}", self.empty_files)?;
        write!(f, "  Total: {}", self.folders + self.files + self.problems + self.empty_files)
    }
}

/// Allow instances of [InsertCount] to be have the `+=` operator used.
impl ops::AddAssign for InsertCount {
    fn add_assign(&mut self, rhs: Self) {
        self.folders += rhs.folders;
        self.files += rhs.files;
        self.problems += rhs.problems;
        self.empty_files += rhs.empty_files;
    }
}

/// Allow two (2) instances of [InsertCount] to be added together with the `+` operator.
impl ops::Add for InsertCount {
    type Output = InsertCount;
    fn add(self, rhs: Self) -> Self::Output {
        InsertCount {
            folders: self.folders + rhs.folders,
            files: self.files + rhs.files,
            problems: self.problems + rhs.problems,
            empty_files: self.empty_files + rhs.empty_files,
        }
    }
}

/// The SQL to reload the filedups table (see `sql/record_duplicate_files.sql`).
const DUPLICATE_FILES_RELOAD_SQL: &str = include_str!("sql/record_duplicate_files.sql");

/// The SQL query to count the number of rows in the filedups table
const DUPLICATE_FILES_ROW_COUNT_SQL: &str = "SELECT COUNT(*) FROM filedups";

/// Initializes the duplicate filenames table.
///
/// It utilizes the [DUPLICATE_FILES_RELOAD_SQL] sql to initialize the table.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used.
pub(crate) fn file_duplicates_reload(conn: &sql::Connection) -> Result<u64> {
    log::debug!("load filedups table");
    conn.execute_batch(DUPLICATE_FILES_RELOAD_SQL)?;
    let row_count = conn.query_row(DUPLICATE_FILES_ROW_COUNT_SQL, (), |row| row.get(0))?;
    Ok(row_count)
}
