//! # Filesystem metadata query module.
//!
//! This is where all the query function live. The SQL is externalized into file
//! within the `sql` subfolder. The `include_str!` macro is used to load them into
//! code at compile time.
//!
use std::collections::BTreeMap;

use super::{
    domain::{DuplicateIds, FileMd, FolderMd, Metadata, ProblemMd},
    Error, PathBuf, Result, EMPTY_FOLDER_FILENAME, ROOT_FOLDER_PARENT_ID,
};
use rusqlite as sql;

/// The SQL to query for what filesystem folders have been added to the database (see `sql/query_root_folder_contents.sql`).
const ROOT_FOLDER_CONTENT_SQL: &str = include_str!("sql/query_root_folder_content.sql");

/// Query for what filesystem folders have been added to the database.
///
/// It uses the [ROOT_FOLDER_CONTENT_SQL] query to locate the folders.
///
/// # Arguments
///
/// * `conn` the database connection that will be used for the query.
/// * `folder_callback` a function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn root_folder_content_query<F>(conn: &sql::Connection, folder_callback: F) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut stmt = conn.prepare_cached(ROOT_FOLDER_CONTENT_SQL)?;
    let mapper = FolderFileRowMap::new(&stmt)?;
    let mut rows = stmt.query(&[(":parent_id", &ROOT_FOLDER_PARENT_ID.to_string())])?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The SQL to query for folder content by its filename (see `sql/query_folder_content_by_name.sql`).
const FOLDER_CONTENT_BY_NAME_SQL: &str = include_str!("sql/query_folder_content_by_name.sql");

/// Query for folder content that have a specific filename.
///
/// It uses the [FOLDER_CONTENT_BY_NAME_SQL] query to locate the folders. Each folder matching the
/// filename will be returned.
///
/// # Arguments
///
/// * `conn` the database connection that will be used for the query.
/// * `folder_name` the folder filename.
/// * `folder_callback` a function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn folder_content_by_name_query<F>(
    conn: &sql::Connection,
    folder_name: &str,
    folder_callback: F,
) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut stmt = conn.prepare_cached(FOLDER_CONTENT_BY_NAME_SQL)?;
    let mapper = FolderFileRowMap::new(&stmt)?;
    let mut rows = stmt.query(&[(":folder_name", folder_name)])?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The SQL to query for folder hierarchies by folder name (see `sql/query_folder_tree_by_name.sql`).
const FOLDER_TREE_BY_NAME_SQL: &str = include_str!("sql/query_folder_tree_by_name.sql");

/// Query for folder hierarchies by a folder name.
///
/// It uses the [FOLDER_TREE_BY_NAME_SQL] query to locate the folders. Each folder matching the
/// filename will be returned.
///
/// # Arguments
///
/// * `conn` the database connection that will be used for the query.
/// * `folder_name` the folder filename.
/// * `folder_callback` a function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn folder_tree_by_name_query<F>(conn: &sql::Connection, folder_name: &str, folder_callback: F) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut statement = conn.prepare_cached(FOLDER_TREE_BY_NAME_SQL)?;
    let mapper = FolderFileRowMap::new(&statement)?;
    let mut rows = statement.query(&[(":folder_name", folder_name)])?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The SQL to query for folder content by pathname (see `sql/query_folder_content_by_pathname.sql`).
const FOLDER_CONTENT_BY_PATHNAME_SQL: &str = include_str!("sql/query_folder_content_by_pathname.sql");

/// Query for a folders content by the pathname.
///
/// It uses the [FOLDER_CONTENT_BY_PATHNAME_SQL] query to locate the folders. Each folder matching the
/// filename will be returned.
///
/// # Arguments
///
/// * `conn` the database connection that will be used for the query.
/// * `folder_name` the folder pathname.
/// * `folder_callback` a function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn folder_content_by_pathname_query<F>(
    conn: &sql::Connection,
    folder_pathname: &str,
    folder_callback: F,
) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut stmt = conn.prepare_cached(FOLDER_CONTENT_BY_PATHNAME_SQL)?;
    let mapper = FolderFileRowMap::new(&stmt)?;
    let mut rows = stmt.query(&[(":folder_pathname", folder_pathname)])?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The SQL to query for a folder hierarchy by a specific pathname (see `sql/query_folder_tree_by_pathname.sql`).
const FOLDER_TREE_BY_PATHNAME_SQL: &str = include_str!("sql/query_folder_tree_by_pathname.sql");

/// Query for a folder hierarchy by a pathname.
///
/// It uses the [FOLDER_TREE_BY_PATHNAME_SQL] query to locate the folders. Each folder in the hierarchy
/// will be returned.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
/// * `folder_name` is the folders pathname.
/// * `folder_callback` is the function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn folder_tree_by_pathname_query<F>(
    conn: &sql::Connection,
    folder_pathname: &str,
    folder_callback: F,
) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut statement = conn.prepare_cached(FOLDER_TREE_BY_PATHNAME_SQL)?;
    let mapper = FolderFileRowMap::new(&statement)?;
    let mut rows = statement.query(&[(":folder_pathname", folder_pathname)])?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The SQL to query for a table row counts (see `sql/query_row_counts.sql`).
const ROW_COUNTER_QUERY: &str = include_str!("sql/query_row_counts.sql");

/// Query for table row counts.
///
/// It uses the [ROW_COUNTER_QUERY] query to retrieve the table counts. The counts are return
/// as a tuple as folder count, file count, and problem count in that order.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
pub(crate) fn get_table_counts_query(conn: &sql::Connection) -> Result<(u64, u64, u64)> {
    let mut stmt = conn.prepare(ROW_COUNTER_QUERY)?;
    let total_folders_idx = stmt.column_index("total_folders")?;
    let total_files_idx = stmt.column_index("total_files")?;
    let total_problems_idx = stmt.column_index("total_problems")?;

    let mut rows = stmt.query(())?;
    match rows.next()? {
        Some(row) => {
            let total_folders = row.get(total_folders_idx)?;
            let total_files = row.get(total_files_idx)?;
            let total_problems = row.get(total_problems_idx)?;
            Ok((total_folders, total_files, total_problems))
        }
        None => Err(Error::from(format!("db: row_counter.sql did not return a row!!!"))),
    }
}

/// The SQL to query for folder pathnames that have a specifi parent id.
const PARENT_FOLDER_PATHNAME_QUERY: &str = "SELECT pathname from folders where parent_id = :parent_id";

/// Query for root folder pathnames.
///
/// It uses the [PARENT_FOLDER_PATHNAME_QUERY] query to retrieve the root pathnames.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
pub(crate) fn root_folders_pathname_query(conn: &sql::Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(PARENT_FOLDER_PATHNAME_QUERY)?;
    let mut rows = stmt.query(&[(":parent_id", &ROOT_FOLDER_PARENT_ID.to_string())])?;
    let mut root_folders: Vec<String> = vec![];
    while let Some(row) = rows.next()? {
        let root_folder: String = row.get(0)?;
        root_folders.push(root_folder);
    }
    Ok(root_folders)
}

/// The SQL to query the database for it's allocated disk size.
const DB_SIZE_QUERY: &str = "SELECT page_count * page_size AS size FROM pragma_page_count(), pragma_page_size()";

/// Query the database for it's allocated size.
///
/// It uses the [DB_SIZE_QUERY] query to retrieve the metrics.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
pub(crate) fn database_metrics_query(conn: &sql::Connection) -> Result<u64> {
    let db_size = conn.query_row(DB_SIZE_QUERY, (), |row| row.get(0))?;
    Ok(db_size)
}

/// The SQL to query for problems (see `sql/query_problems.sql`).
const PROBLEMS_QUERY: &str = include_str!("sql/query_problems.sql");

/// Query the problems that have occurred.
///
/// It uses the [PROBLEMS_QUERY] query to retrieve the problems.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
pub(crate) fn problems_query(conn: &sql::Connection) -> Result<Vec<Metadata>> {
    let mut stmt = conn.prepare_cached(PROBLEMS_QUERY)?;
    let mapper = ProblemsMap::new(&stmt)?;
    let mut rows = stmt.query(())?;
    Ok(mapper.to_problems(&mut rows)?)
}

/// The data mapper for results returned from the problems query result set.
///
/// The attributes of the structure hold the column index in the result set for the metadata attributes.
#[derive(Debug)]
struct ProblemsMap {
    /// The index of the folder identifier.
    pub folder_id: usize,
    /// The index of the parent folder identifier.
    pub folder_parent_id: usize,
    /// The index of the folder pathname.
    pub folder_pathname: usize,
    /// The index of the problem identifier.
    pub problem_id: usize,
    /// The index of the problem parent identifier
    pub problem_parent_id: usize,
    /// The index of the pathname of the problem.
    pub problem_pathname: usize,
    /// The index of the description of the problem.
    pub problem_description: usize,
}
impl ProblemsMap {
    /// Creates a new instance of the problem mapper.
    ///
    /// The statement is used to get the column index for metadata being mined.
    /// # Arguments
    ///
    /// * `stmt` the prepared statement being used.
    fn new(stmt: &sql::CachedStatement) -> Result<ProblemsMap> {
        Ok(ProblemsMap {
            folder_id: stmt.column_index("folder_id")?,
            folder_parent_id: stmt.column_index("folder_parent_id")?,
            folder_pathname: stmt.column_index("folder_pathname")?,
            problem_id: stmt.column_index("problem_id")?,
            problem_parent_id: stmt.column_index("problem_parent_id")?,
            problem_pathname: stmt.column_index("problem_pathname")?,
            problem_description: stmt.column_index("problem_description")?,
        })
    }
    /// Creates the folder metadata for a folder that has a problem.
    ///
    /// # Arguments
    ///
    /// * `row` a row from the query result set.
    fn to_folder_problem(&self, row: &sql::Row) -> Result<FolderMd> {
        let mut folder_md = FolderMd {
            id: row.get(self.folder_id)?,
            parent_id: row.get(self.folder_parent_id)?,
            pathname: row.get(self.folder_pathname)?,
            name: String::default(),
            size: 0,
            created: 0,
            modified: 0,
            children: BTreeMap::new(),
        };
        // folder_md.children.push(Metadata::Problem(self.to_problem(row)?));
        let problem = self.to_problem(row)?;
        folder_md.children.insert(problem.name.clone(), Metadata::Problem(problem));
        Ok(folder_md)
    }
    /// Creates the problem metadata.
    ///
    /// # Arguments
    ///
    /// * `row` is a single result from the query results.
    fn to_problem(&self, row: &sql::Row) -> Result<ProblemMd> {
        let problem_pathname = row.get(self.problem_pathname)?;
        let problem_name = PathBuf::from(&problem_pathname).file_name().unwrap().to_str().unwrap().to_string();
        Ok(ProblemMd {
            id: row.get(self.problem_id)?,
            parent_id: row.get(self.problem_parent_id)?,
            pathname: problem_pathname,
            name: problem_name,
            description: row.get(self.problem_description)?,
        })
    }
    /// Converts the results of a query into problem metadata.
    ///
    /// # Arguments
    ///
    /// * `rows`contains the results of a database query.
    fn to_problems(&self, rows: &mut sql::Rows) -> Result<Vec<Metadata>> {
        let mut metadata = vec![];
        let mut folder_md_option: Option<FolderMd> = None;
        while let Some(row) = rows.next()? {
            if folder_md_option.is_none() {
                folder_md_option.replace(self.to_folder_problem(row)?);
            } else {
                let folder_md = folder_md_option.as_mut().unwrap();
                let folder_id: i64 = row.get(self.folder_id).unwrap_or(-1);
                if folder_id == folder_md.id {
                    // if its the same folder add the problem child
                    let problem_md = self.to_problem(row)?;
                    folder_md.children.insert(problem_md.name.clone(), Metadata::Problem(problem_md));
                } else {
                    // save the previous folders problems and setup the new folder
                    let md = folder_md_option.replace(self.to_folder_problem(row)?).unwrap();
                    metadata.push(Metadata::Folder(md));
                }
            }
        }
        // this will be default if there are no problems...
        if let Some(folder_md) = folder_md_option {
            metadata.push(Metadata::Folder(folder_md));
        }
        Ok(metadata)
    }
}

/// The data mapper for results returned from folder/file query result sets.
///
/// The attributes of the structure hold the column index in the result set for the metadata attributes.
struct FolderFileRowMap {
    /// The index of the folder identifier.
    pub folder_id: usize,
    /// The index of the parent identifier for the folder.
    pub folder_parent_id: usize,
    /// The index of the pathname of the folder.
    pub folder_pathname: usize,
    /// The index of the filename of the folder.
    pub folder_name: usize,
    /// The index of the disk size of the folder.
    pub folder_size: usize,
    /// The index of the timestamp for when the folder was created.
    pub folder_created: usize,
    /// The index of the timestamp of when the folder was last modified.
    pub folder_modified: usize,
    /// The index of the file identifier.
    pub file_id: usize,
    /// The index of the file parent indentifier.
    pub file_parent_id: usize,
    /// The index of the file pathname.
    pub file_pathname: usize,
    /// The index of the file filename.
    pub file_name: usize,
    /// The index of the symbolic link indicator.
    pub file_is_symlink: usize,
    /// The index of the file size on disk.
    pub file_size: usize,
    /// The index of the timestamp for when the file was created.
    pub file_created: usize,
    /// The index of the timestamp of when the file was last modified.
    pub file_modified: usize,
}

impl FolderFileRowMap {
    /// Creates a new instance of the folder/file mapper.
    ///
    /// The statement is used to get the column index for metadata being mined.
    /// # Arguments
    ///
    /// * `stmt` is the prepared statement being used.
    fn new(stmt: &sql::CachedStatement) -> Result<FolderFileRowMap> {
        Ok(FolderFileRowMap {
            folder_id: stmt.column_index("folder_id")?,
            folder_parent_id: stmt.column_index("folder_parent_id")?,
            folder_pathname: stmt.column_index("folder_pathname")?,
            folder_name: stmt.column_index("folder_name")?,
            folder_size: stmt.column_index("folder_size")?,
            folder_created: stmt.column_index("folder_created")?,
            folder_modified: stmt.column_index("folder_modified")?,
            file_id: stmt.column_index("file_id")?,
            file_parent_id: stmt.column_index("file_parent_id")?,
            file_pathname: stmt.column_index("file_pathname")?,
            file_name: stmt.column_index("file_name")?,
            file_is_symlink: stmt.column_index("file_is_symlink")?,
            file_size: stmt.column_index("file_size")?,
            file_created: stmt.column_index("file_created")?,
            file_modified: stmt.column_index("file_modified")?,
        })
    }
    /// Converts the row to folder metadata possibly containing the child file metadata
    /// if the folder was not empty.
    ///
    /// # Arguments
    ///
    /// * `row` is a single result from the query results.
    fn to_folder_file(&self, row: &sql::Row) -> Result<FolderMd> {
        let mut folder_md = self.to_folder(row)?;
        if let Some(file_md) = self.to_file(row)? {
            folder_md.children.insert(file_md.name.clone(), Metadata::File(file_md));
        }
        Ok(folder_md)
    }
    /// Converts the row to folder metadata.
    ///
    /// # Arguments
    ///
    /// * `row` is a single result from the query results.
    fn to_folder(&self, row: &sql::Row) -> Result<FolderMd> {
        let folder_md = FolderMd {
            id: row.get(self.folder_id)?,
            parent_id: row.get(self.folder_parent_id)?,
            pathname: row.get(self.folder_pathname)?,
            name: row.get(self.folder_name)?,
            size: row.get(self.folder_size)?,
            created: row.get(self.folder_created)?,
            modified: row.get(self.folder_modified)?,
            children: BTreeMap::new(),
        };
        Ok(folder_md)
    }
    /// Converts the row to file metadata.
    ///
    /// If the filename is [EMPTY_FOLDER_FILENAME] the result will be `None`.
    ///
    /// # Arguments
    ///
    /// * `row` is a single result from the query results.
    fn to_file(&self, row: &sql::Row) -> Result<Option<FileMd>> {
        // a -1 will be returned for the file_id and file_parent id if the query does not include file data
        let file_id: i64 = row.get(self.file_id).unwrap_or(-1);
        let file_parent_id: i64 = row.get(self.file_parent_id).unwrap_or(-1);
        let file_name = row.get(self.file_name).unwrap_or(String::default());
        if file_id == -1 || file_parent_id == -1 || file_name == EMPTY_FOLDER_FILENAME {
            Ok(None)
        } else {
            let file_md = FileMd {
                id: file_id,
                parent_id: file_parent_id,
                pathname: row.get(self.file_pathname)?,
                name: file_name,
                is_symlink: row.get(self.file_is_symlink)?,
                size: row.get(self.file_size)?,
                created: row.get(self.file_created)?,
                modified: row.get(self.file_modified)?,
            };
            Ok(Some(file_md))
        }
    }
    /// Converts the results of a query into folder metadata.
    ///
    /// # Arguments
    ///
    /// * `rows` the rows of a query result set.
    /// * `folder_callback` a function that will be called with folder metadata properties. The function
    /// will be called once for each resulting folder found. If `false` is returned from the function
    /// iteration over the result set will stop.
    fn to_folders<F>(&self, rows: &mut sql::Rows, mut folder_callback: F) -> Result<()>
    where
        // Now I'm starting to question if the function shouldn't return a standard
        // result with a string as an error. Either that or get over the need to bring
        // in the db Error to consummers of the function.
        F: FnMut(FolderMd) -> Result<bool>,
    {
        let mut folder_md_option: Option<FolderMd> = None;
        while let Some(row) = rows.next()? {
            if folder_md_option.is_none() {
                folder_md_option.replace(self.to_folder_file(row)?);
            } else {
                let folder_md = folder_md_option.as_mut().unwrap();
                let folder_id: i64 = row.get(self.folder_id).unwrap_or(-1);
                // check to see if the folder is changing
                if folder_md.id != folder_id {
                    // create the new folder metadata and give the previous one to the caller
                    let md = folder_md_option.replace(self.to_folder_file(row)?).unwrap();
                    if !folder_callback(md)? {
                        folder_md_option = None;
                        break;
                    }
                } else if let Some(file_md) = self.to_file(row)? {
                    // add the file to the parent folder
                    folder_md.children.insert(file_md.name.clone(), Metadata::File(file_md));
                }
            }
        }
        if let Some(md) = folder_md_option {
            folder_callback(md)?;
        }
        Ok(())
    }
}

/// The SQL query to count the number of duplicate filenames
const COUNT_DUPLICATE_FILENAMES: &str =
    "SELECT COUNT(DISTINCT files.name) FROM filedups JOIN files ON file_id = files.id";

/// The SQL query to count the number of folders that have duplicate filenames
const COUNT_DUPLICATE_FILENAME_FOLDERS: &str =
    "SELECT COUNT(DISTINCT folders.name) FROM filedups JOIN folders ON filedups.parent_id = folders.id";

/// Retreive the count of unique folders and filenames from the `filedups` table.
///
/// The tuple returned contains a count of the unique folder names and the unique filenames
/// respectively.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
pub(crate) fn duplicate_file_metrics(conn: &sql::Connection) -> Result<(u64, u64)> {
    let folders = conn.query_row(COUNT_DUPLICATE_FILENAME_FOLDERS, (), |row| row.get(0))?;
    let filenames = conn.query_row(COUNT_DUPLICATE_FILENAMES, (), |row| row.get(0))?;
    Ok((folders, filenames))
}

/// The query to find the folder and file identifiers for duplicate filenames.
const DUPLICATE_FILE_IDS_SQL: &str = include_str!("sql/query_filedups_ids.sql");

/// Get the folder and file identifiers for duplicate filenames.
///
/// # Arguments
///
/// * `conn` is the database connection that will be used for the query.
/// * `ids_callback` is the function that will be called with folder metadata properties.
pub(crate) fn duplicate_ids<F>(conn: &sql::Connection, ids_callback: F) -> Result<()>
where
    F: FnMut(DuplicateIds) -> Result<bool>,
{
    let mut statement = conn.prepare_cached(DUPLICATE_FILE_IDS_SQL)?;
    let mapper = DuplicateIdMapper::new(&statement)?;
    let mut rows = statement.query(())?;
    mapper.to_ids(&mut rows, ids_callback)?;
    Ok(())
}

/// The SQL to query for problems (see `sql/query_problems.sql`).
const DUPLICATE_FILES_METADATA_SQL: &str = include_str!("sql/query_filedups_metadata.sql");

/// Query the metadata for duplicate filenames.
///
/// It uses the [DUPLICATE_FILES_METADATA_SQL] query to get the metadata
///
/// # Arguments
///
/// * `conn` the database connection that will be used for the query.
/// * `folder_callback` is the function that will be called with folder metadata properties. The function
/// will be called once for each resulting folder found. If `false` is returned from the function
/// iteration over the result set will stop.
pub(crate) fn duplicate_files_metadata_query<F>(conn: &sql::Connection, folder_callback: F) -> Result<()>
where
    F: FnMut(FolderMd) -> Result<bool>,
{
    let mut stmt = conn.prepare_cached(DUPLICATE_FILES_METADATA_SQL)?;
    let mapper = FolderFileRowMap::new(&stmt)?;
    let mut rows = stmt.query(())?;
    mapper.to_folders(&mut rows, folder_callback)?;
    Ok(())
}

/// The mapper that consummes the duplicate files identifiers query.
struct DuplicateIdMapper {
    /// The index of the filename.
    filename: usize,
    /// The index of the associated file identifier.
    file_id: usize,
    /// The index of the associated folder identifier.
    parent_id: usize,
}
impl DuplicateIdMapper {
    /// Creates a new instance of the duplicate file identifiers mapper.
    ///
    /// The statement is used to get the column index for metadata being mined.
    /// # Arguments
    ///
    /// * `stmt` is the prepared statement being used.
    fn new(stmt: &sql::CachedStatement) -> Result<DuplicateIdMapper> {
        Ok(DuplicateIdMapper {
            filename: stmt.column_index("filename")?,
            file_id: stmt.column_index("file_id")?,
            parent_id: stmt.column_index("parent_id")?,
        })
    }
    /// Converts the results of a query into duplicate file identifiers.
    ///
    /// # Arguments
    ///
    /// * `rows` the rows of a query result set.
    /// * `ids_callback` is a function that will be called with folder metadata properties. The function
    /// will be called once for each resulting folder found. If `false` is returned from the function
    /// iteration over the result set will stop.
    fn to_ids<F>(&self, rows: &mut sql::Rows, mut ids_callback: F) -> Result<()>
    where
        F: FnMut(DuplicateIds) -> Result<bool>,
    {
        let mut duplicate_ids_option: Option<DuplicateIds> = None;
        while let Some(row) = rows.next()? {
            let filename: String = row.get(self.filename)?;
            if duplicate_ids_option.is_none() {
                duplicate_ids_option.replace(DuplicateIds::new(&filename));
            } else if duplicate_ids_option.as_ref().unwrap().filename != filename {
                // theres a new filename so hand off the previous metadata and create the new one
                let duplicate_ids = duplicate_ids_option.replace(DuplicateIds::new(&filename)).unwrap();
                if !ids_callback(duplicate_ids)? {
                    duplicate_ids_option = None;
                    break;
                }
            }
            let parent_id: i64 = row.get(self.parent_id)?;
            let file_id: i64 = row.get(self.file_id)?;
            duplicate_ids_option.as_mut().unwrap().add(parent_id, file_id);
        }
        if let Some(duplicate_ids) = duplicate_ids_option {
            ids_callback(duplicate_ids)?;
        }
        Ok(())
    }
}

// this needs to play on both Windoz and Unux
#[cfg(test)]
#[cfg(windows)]
mod windows_tests {
    use super::super::{
        database_connection,
        filesys::{collect_metadata, FsMetadata},
        load_fs_metadata, schema_init,
    };
    use super::*;

    fn test_db_connection(testcase_option: Option<&FsMetadata>) -> Result<sql::Connection> {
        let mut conn = database_connection(None)?;
        schema_init(&conn).unwrap();
        if let Some(fs_metadata) = testcase_option {
            load_fs_metadata(&mut conn, &fs_metadata)?;
        }
        Ok(conn)
    }

    #[test]
    fn folder_content() {
        let testcase_data = include_str!("query/folder_content_testcase.yaml");
        let fs_metadata: FsMetadata = serde_yaml::from_str(testcase_data).unwrap();
        let conn = test_db_connection(Some(&fs_metadata)).unwrap();

        // explicitly verify the query has all the needed columns
        let stmt = conn.prepare_cached(super::FOLDER_CONTENT_BY_NAME_SQL).unwrap();
        super::FolderFileRowMap::new(&stmt).unwrap();

        // now verify content with a single root
        let mut folders = vec![];
        super::folder_content_by_name_query(&conn, "testcase", |folder_md| {
            folders.push(folder_md);
            Ok(true)
        })
        .unwrap();
        assert_eq!(folders.len(), 3);
        assert_eq!(folders[0].children.len(), 1);
        assert!(folders[0].children.get("some_file.dat").is_some());
        assert_eq!(folders[1].children.len(), 0);
        assert_eq!(folders[1].name, "folder1");
        assert_eq!(folders[2].children.len(), 0);
        assert_eq!(folders[2].name, "folder3");
    }

    #[test]
    fn query_folder_tree() {
        // collect_fs_metadata(
        //     PathBuf::from(r"g:\dev\playground\disktool\src"),
        //     PathBuf::from(r"src\db\query_folder_testcase.yaml")).unwrap();
        let testcase_data = include_str!("query/query_folder_testcase.yaml");
        let fs_metadata: FsMetadata = serde_yaml::from_str(testcase_data).unwrap();
        let conn = test_db_connection(Some(&fs_metadata)).unwrap();

        // explicitly verify the query has all the needed columns
        let stmt = conn.prepare_cached(super::FOLDER_TREE_BY_NAME_SQL).unwrap();
        super::FolderFileRowMap::new(&stmt).unwrap();

        let mut folders: Vec<FolderMd> = vec![];
        super::folder_tree_by_name_query(&conn, "src", |folder| {
            folders.push(folder);
            Ok(true)
        })
        .unwrap();
        // let folders = super::folder_tree_query(&conn, Some("src".to_string()), true).unwrap();
        assert_eq!(folders.len(), 5);
        let folder = &folders[0];
        assert_eq!(folder.name, "src");
        assert_eq!(folder.children.len(), 5);
        let folder = &folders[1];
        assert_eq!(folder.name, "cli");
        assert_eq!(folder.children.len(), 4);
        let folder = &folders[2];
        assert_eq!(folder.name, "domain");
        assert_eq!(folder.children.len(), 2);
        let folder = &folders[3];
        assert_eq!(folder.name, "db");
        assert_eq!(folder.children.len(), 16);
        let folder = &folders[4];
        assert_eq!(folder.name, "empty");
        assert!(folder.children.is_empty());
    }

    /// a utility to create a YAML test case from a filesystem directory
    #[allow(dead_code)]
    fn collect_fs_metadata(folder: PathBuf, output_file: PathBuf) -> super::Result<()> {
        use std::fs::File;
        use std::io::Write;
        let fs_metadata = collect_metadata(&folder).unwrap();
        let yaml = serde_yaml::to_string(&fs_metadata).unwrap();
        let mut file = File::create(output_file).unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{database_connection, schema_init};
    use super::*;

    fn test_db_connection() -> sql::Connection {
        let conn = database_connection(None).expect("Error creating Connection!!!");
        schema_init(&conn).expect("Error initializing schema!!!");
        conn
    }

    #[test]
    fn root_folder_content_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(ROOT_FOLDER_CONTENT_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }

    #[test]
    fn folder_content_by_name_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(FOLDER_CONTENT_BY_NAME_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }

    #[test]
    fn folder_tree_by_name_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(FOLDER_TREE_BY_NAME_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }

    #[test]
    fn folder_content_by_pathname_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(FOLDER_CONTENT_BY_PATHNAME_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }

    #[test]
    fn folder_tree_by_pathname_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(FOLDER_TREE_BY_PATHNAME_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }

    #[test]
    fn problems_query() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(PROBLEMS_QUERY).unwrap();
        ProblemsMap::new(&stmt).unwrap();
    }

    #[test]
    fn duplicate_ids_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(DUPLICATE_FILE_IDS_SQL).unwrap();
        DuplicateIdMapper::new(&stmt).unwrap();
    }

    #[test]
    fn duplicate_files_metadata_query_sql() {
        let conn = test_db_connection();
        let stmt = conn.prepare_cached(DUPLICATE_FILES_METADATA_SQL).unwrap();
        FolderFileRowMap::new(&stmt).unwrap();
    }
}
