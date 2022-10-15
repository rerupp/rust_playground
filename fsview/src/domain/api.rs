use std::path::PathBuf;

use super::{db, filesys, sql, DbInformation, Result, HierarchyBuilder, Metadata};

pub fn get_folder_by_name(conn: &sql::Connection, folder_name: &str, recursive: bool) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = HierarchyBuilder::default();
    if recursive {
        db::folder_tree_by_name_query(conn, folder_name, |folder| {
            hierarchy_builder.add(folder);
            Ok(true)
        })?
    } else {
        db::folder_content_by_name_query(conn, folder_name, |folder| {
            hierarchy_builder.add(folder);
            Ok(true)
        })?
    }
    Ok(hierarchy_builder.get())
}

pub fn get_folder_by_pathname(conn: &sql::Connection, folder_pathname: &str, recursive: bool) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = HierarchyBuilder::default();
    if recursive {
        db::folder_tree_by_pathname_query(conn, folder_pathname, |folder| {
            hierarchy_builder.add(folder);
            Ok(true)
        })?
    } else {
        db::folder_content_by_pathname_query(conn, folder_pathname, |folder| {
            hierarchy_builder.add(folder);
            Ok(true)
        })?
    }
    Ok(hierarchy_builder.get())
}

pub fn get_root_content(conn: &sql::Connection) -> Result<Vec<Metadata>> {
    let mut hierarchy_builder = HierarchyBuilder::default();
    db::root_folder_content_query(conn, |folder| {
        hierarchy_builder.add(folder);
        Ok(true)
    })?;
    Ok(hierarchy_builder.get())
}

pub fn get_db_information(conn: &sql::Connection) -> Result<DbInformation> {
    let root_folders = db::root_folders_pathname_query(conn)?;
    let (folder_count, file_count, problem_count) = db::get_table_counts_query(conn)?;
    let database_size = db::database_metrics_query(conn)?;
    // TODO: add empty rows to db information
    Ok(DbInformation {
        root_folders,
        file_count,
        folder_count,
        problem_count,
        database_size,
    })
}

pub fn get_problems(conn: &sql::Connection) -> Result<Vec<Metadata>> {
    Ok(db::problems_query(conn)?)
}

pub fn initialize_db(conn: &sql::Connection, drop_database: bool) -> Result<()> {
    if drop_database {
        db::schema_drop(conn, true)?;
    }
    Ok(db::schema_init(conn)?)
}

pub fn add_filesystem_folder(mut conn: sql::Connection, folder_pathname: &PathBuf) -> Result<()> {
    let folder = filesys::collect_metadata(&folder_pathname)?;
    if log::log_enabled!(log::Level::Trace) {
        log::trace!("{} entries found...", filesys::count_metadata(&folder));
    }
    Ok(db::load_fs_metadata(&mut conn, &folder)?)
}
