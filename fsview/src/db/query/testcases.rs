//! These tests are really integeration tests for the db module. Because db is not exposed from
//! the library they need to go here in order to have access to the db module functions.
//! 
use super::*;
use super::filesys::{collect_metadata, FsMetadata};
use std::fs::File;
use std::io::Write;

fn test_db_connection(testcase_option: Option<&FsMetadata>) -> Result<sql::Connection> {
    let mut conn = database_connection(Some(&PathBuf::from("test.db")))?;
    // let mut conn = sql::Connection::open_in_memory()?;
    schema_drop(&conn, true).unwrap();
    schema_init(&conn).unwrap();
    if let Some(fs_metadata) = testcase_option {
        super::load_fs_metadata(&mut conn, &fs_metadata)?;
    }
    Ok(conn)
}

#[test]
fn folder_content() {
    // collect_fs_metadata(
    //     PathBuf::from(r#"g:\testcase"#),
    //     PathBuf::from(r#"src\db\folder_content_testcase.yaml"#)).unwrap();
    let testcase_data = include_str!("folder_content_testcase.yaml");
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
    assert_eq!(folders[0].children[0].name(), "some_file.dat");
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
    let testcase_data = include_str!("query_folder_testcase.yaml");
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
    let fs_metadata = collect_metadata(&folder).unwrap();
    let yaml = serde_yaml::to_string(&fs_metadata).unwrap();
    let mut file = File::create(output_file).unwrap();
    file.write_all(yaml.as_bytes()).unwrap();
    Ok(())
}
