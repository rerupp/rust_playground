//! The internal functions used to implement the domain session.
use std::path::PathBuf;

use super::StopWatch;

use super::{
    db, filesys, sql, DbInformation, DuplicateFolders, DuplicateFoldersBuilder, FolderMd, DuplicateFoldersMatch, FoldersNoMatch,
    Metadata, Result,
};

/// Get metadata for a folder by its filename.
///
/// The list of metadata will contain multiple entries if the folder name is found multiple
/// times.
///
/// # Arguments
///
/// * `conn` is the database connection.
/// * `folder_name` is the name of the folder that will be searched for.
/// * `recursive` if `true` will traverse the folders hierarchy.
pub(crate) fn get_folder_by_name(conn: &sql::Connection, folder_name: &str, recursive: bool) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = hierarchy::Builder::new();
    if recursive {
        db::folder_tree_by_name_query(conn, folder_name, |folder_md| {
            hierarchy_builder.add(folder_md);
            Ok(true)
        })?
    } else {
        db::folder_content_by_name_query(conn, folder_name, |folder_md| {
            hierarchy_builder.add(folder_md);
            Ok(true)
        })?
    }
    Ok(hierarchy_builder.get())
}

/// Get metadata for a folder by its pathname.
///
/// # Arguments
///
/// * `conn` is the database connection.
/// * `folder_pathname` is the name of the folder that will be searched for.
/// * `recursive` if `true` will traverse the folders hierarchy.
pub(crate) fn get_folder_by_pathname(
    conn: &sql::Connection,
    folder_pathname: &str,
    recursive: bool,
) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = hierarchy::Builder::new();
    if recursive {
        db::folder_tree_by_pathname_query(conn, folder_pathname, |folder_md| {
            hierarchy_builder.add(folder_md);
            Ok(true)
        })?
    } else {
        db::folder_content_by_pathname_query(conn, folder_pathname, |folder_md| {
            hierarchy_builder.add(folder_md);
            Ok(true)
        })?
    }
    Ok(hierarchy_builder.get())
}

/// Get metadata for the root folders.
///
/// The top level directory for each filesystem directory added is considered the root
/// folder. At some point I might add functionality to recurse the folder structure but
/// for now you can't do that.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn get_root_content(conn: &sql::Connection) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = hierarchy::Builder::new();
    db::root_folder_content_query(conn, |folder_md| {
        hierarchy_builder.add(folder_md);
        Ok(true)
    })?;
    Ok(hierarchy_builder.get())
}

/// Get metadata concerning the database storage.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn get_db_information(conn: &sql::Connection) -> Result<DbInformation> {
    let root_folders = db::root_folders_pathname_query(conn)?;
    let (folder_count, file_count, problem_count) = db::get_table_counts_query(conn)?;
    let database_size = db::database_metrics_query(conn)?;
    // TODO: add empty rows to db information
    Ok(DbInformation { root_folders, file_count, folder_count, problem_count, database_size })
}

/// Get the metadata describing problems that might have occurred loading filesystem directories
/// and files.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn get_problems(conn: &sql::Connection) -> Result<Vec<Metadata>> {
    Ok(db::problems_query(conn)?)
}

/// Initialize the database.
///
/// # Arguments
///
/// * `conn` is the database connection.
/// * `drop_database` if `true` will drop the database before applying the initialization SQL.
pub(crate) fn initialize_db(conn: &sql::Connection, drop_database: bool) -> Result<()> {
    if drop_database {
        db::schema_drop(conn, true)?;
    }
    db::schema_init(conn)?;
    Ok(())
}

/// Add a filesystem folder structure.
///
/// # Arguments
///
/// * `conn` is the database connection.
/// * `folder_pathname` is the name of the filesystem folder that will be loaded.
pub(crate) fn add_filesystem_folder(mut conn: sql::Connection, folder_pathname: &PathBuf) -> Result<()> {
    let folder = filesys::collect_metadata(&folder_pathname)?;
    if log::log_enabled!(log::Level::Trace) {
        log::trace!("{} entries found...", filesys::count_metadata(&folder));
    }
    db::load_fs_metadata(&mut conn, &folder)?;
    Ok(())
}

/// Load the data that supports identifying duplicate files.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn file_duplicates_reload(conn: &sql::Connection) -> Result<u64> {
    Ok(db::file_duplicates_reload(conn)?)
}

/// Get the metadata describing the duplicate files that were found.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn file_duplicates_summary(conn: &sql::Connection) -> Result<(u64, u64)> {
    Ok(db::duplicate_file_metrics(conn)?)
}

/// Get the metadata describing details about duplicate files that were found.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn duplicate_folders_metadata(conn: &sql::Connection) -> Result<DuplicateFolders> {
    let mut builder = DuplicateFoldersBuilder::new();
    let mut stopwatch = StopWatch::start_new();
    db::duplicate_files_metadata_query(conn, |md| {
        builder.add_folder_md(md);
        Ok(true)
    })?;
    log::info!("duplicate folder metadata load: {stopwatch}");
    stopwatch.reset().start();
    db::duplicate_ids(conn, |md| {
        builder.add_duplicate_ids(md);
        Ok(true)
    })?;
    log::info!("dupldate folder filenames load: {stopwatch}");
    stopwatch.reset().start();
    let duplicate_folders = builder.build()?;
    log::info!("duplicate folders build: {stopwatch}");
    Ok(duplicate_folders)
}

/// Get the metadata describing details about duplicate files that were found.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn folders_match_metadata(conn: &sql::Connection) -> Result<DuplicateFoldersMatch> {
    let duplicate_folders = duplicate_folders_metadata(conn)?;
    let elapsed = StopWatch::start_new();
    let folders_match = DuplicateFoldersMatch::from(duplicate_folders);
    log::info!("folders file match: {}", elapsed);
    Ok(folders_match)
}

/// Get metadata describing folders that were not part of a folder group match.
///
/// # Arguments
///
/// * `conn` is the database connection.
pub(crate) fn folders_no_match_metadata(conn: &sql::Connection) -> Result<FoldersNoMatch> {
    let duplicate_folders = duplicate_folders_metadata(conn)?;
    let elapsed = StopWatch::start_new();
    let folders_no_match = FoldersNoMatch::from(duplicate_folders);
    log::info!("folders file match: {}", elapsed);
    Ok(folders_no_match)
}

mod hierarchy {
    //! Contains the builder that creates a folders structure.
    use super::*;

    /// The metadata associated with re-creating a folders structure.
    pub struct Builder {
        /// The list of folder metadata.
        pub metadata_folders: Vec<Metadata>,
        /// The current folder hierarchy being traversed.
        current_hierarchy: Vec<FolderMd>,
    }
    impl Builder {
        /// Create a new instance of the builder.
        pub fn new() -> Self {
            Self { metadata_folders: vec![], current_hierarchy: vec![] }
        }
        /// Add some folder metadata to the builder.
        ///
        /// # Arguments
        ///
        /// * `folder_md` is the folder metadata being added.
        pub fn add(&mut self, folder_md: FolderMd) {
            // if the current hierarchy is empty then this is the start of a new one
            if self.current_hierarchy.is_empty() {
                self.current_hierarchy.push(folder_md);
            } else {
                // if the folder metadata is a child push it onto the current hierarchy
                let current_parent_id = self.current_hierarchy.last().map_or(0, |md| md.id);
                if folder_md.parent_id == current_parent_id {
                    self.current_hierarchy.push(folder_md);
                } else {
                    // if not then the current folder metdata is complete
                    let child_md = self.current_hierarchy.pop().unwrap();
                    // child_md.children.sort_by(|lhs, rhs| lhs.name().cmp(rhs.name()));

                    // if the current hierarchy is empty it's a new metadata root folder
                    if self.current_hierarchy.is_empty() {
                        self.metadata_folders.push(Metadata::Folder(child_md));
                    } else {
                        // otherwise it is a child of the current hierarchy so save it and keep looking
                        let parent_md = self.current_hierarchy.last_mut().unwrap();
                        parent_md.children.insert(child_md.name.clone(), Metadata::Folder(child_md));
                    }
                    self.add(folder_md);
                }
            }
        }
        /// An internal function that makes sure the current hierarchy is consummed.
        fn flush(&mut self) {
            while !self.current_hierarchy.is_empty() {
                let current_md = self.current_hierarchy.pop().unwrap();
                if self.current_hierarchy.is_empty() {
                    self.metadata_folders.push(Metadata::Folder(current_md))
                } else {
                    let parent_md = self.current_hierarchy.last_mut().unwrap();
                    parent_md.children.insert(current_md.name.clone(), Metadata::Folder(current_md));
                }
            }
        }
        /// Get the metadata that makes up a folders hierarchy.
        ///
        /// The list will contain multiple folders for something like finding a folder by name.
        pub fn get(mut self) -> Vec<Metadata> {
            self.flush();
            if log::log_enabled!(log::Level::Trace) {
                for metadata in &self.metadata_folders {
                    dump_metadata(metadata);
                }
            }
            self.metadata_folders
        }
    }

    /// An internal function that traverses the metadata structure and logs the contents.
    ///
    /// `trace` level logging must be in effect to see the output.
    ///
    /// # Arguments
    ///
    /// `node` is the metadata that will be traversed.
    fn dump_metadata(node: &Metadata) {
        match node {
            Metadata::Problem(problem_md_) => log::trace!("Problem: {}", problem_md_.pathname),
            Metadata::File(file_md) => {
                let file_type = if file_md.is_symlink { "Symlink" } else { "File" };
                log::trace!("{file_type}: {}", file_md.pathname)
            }
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => {
                let folder_type = if node.is_root() { "Root" } else { "Folder" };
                log::trace!("{folder_type}: {}", folder_md.pathname);
                for child_node in folder_md.children.values() {
                    dump_metadata(child_node)
                }
            }
        }
    }
}
