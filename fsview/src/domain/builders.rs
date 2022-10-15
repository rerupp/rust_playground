// use super::*;
#[cfg(test)]
use super::{db, filesys, PathBuf};
use super::{FolderMd, Metadata};

#[derive(Default)]
pub struct Hierarchy {
    pub metadata_folders: Vec<Metadata>,
    current_hierarchy: Vec<FolderMd>,
}

#[allow(dead_code)]
impl Hierarchy {
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
                let mut child_md = self.current_hierarchy.pop().unwrap();
                child_md.children.sort_by(|lhs, rhs| lhs.name().cmp(rhs.name()));

                // if the current hierarchy is empty it's a new metadata root folder
                if self.current_hierarchy.is_empty() {
                    self.metadata_folders.push(Metadata::Folder(child_md));
                } else {
                    // otherwise it is a child of the current hierarchy so save it and keep looking
                    let parent_md = self.current_hierarchy.last_mut().unwrap();
                    parent_md.children.push(Metadata::Folder(child_md));
                }
                self.add(folder_md);
            }
        }
    }

    fn flush(&mut self) {
        while !self.current_hierarchy.is_empty() {
            let mut current_md = self.current_hierarchy.pop().unwrap();
            current_md.children.sort_by(|lhs, rhs| lhs.name().cmp(rhs.name()));
            let folder_metadata = Metadata::Folder(current_md);

            if self.current_hierarchy.is_empty() {
                self.metadata_folders.push(folder_metadata)
            } else {
                let parent_md = self.current_hierarchy.last_mut().unwrap();
                parent_md.children.push(folder_metadata);
            }
        }
    }

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

pub fn dump_metadata(node: &Metadata) {
    match node {
        Metadata::Problem(problem_md_) => log::trace!("Problem: {}", problem_md_.pathname),
        Metadata::File(file_md) => {
            let file_type = if file_md.is_symlink { "Symlink" } else { "File" };
            log::trace!("{file_type}: {}", file_md.pathname)
        }
        Metadata::Root(folder_md) | Metadata::Folder(folder_md) => {
            let folder_type = if node.is_root() { "Root" } else { "Folder" };
            log::trace!("{folder_type}: {}", folder_md.pathname);
            for child_node in &folder_md.children {
                dump_metadata(child_node)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rusqlite as sql;
    use std::fs::File;
    use std::io::Write;

    fn test_db_connection(testcase_option: Option<&filesys::FsMetadata>) -> sql::Connection {
        let mut conn = db::database_connection(Some(&PathBuf::from("test.db"))).unwrap();
        // let mut conn = sql::Connection::open_in_memory().unwrap();
        db::schema_drop(&conn, true).unwrap();
        db::schema_init(&conn).unwrap();
        if let Some(fs_metadata) = testcase_option {
            db::load_fs_metadata(&mut conn, &fs_metadata).unwrap();
        }
        conn
    }

    /// a utility to create a YAML test case from a filesystem directory
    #[allow(dead_code)]
    fn collect_fs_metadata(folder: PathBuf, output_file: PathBuf) {
        let fs_metadata = filesys::collect_metadata(&folder).unwrap();
        let yaml = serde_yaml::to_string(&fs_metadata).unwrap();
        let mut file = File::create(output_file).unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
    }

    // #[test]
    // fn metadata() {
    //     let metadata = super::Metadata::Folder(super::FolderMd::new(&super::DbFolder {
    //         id: 50,
    //         parent_id: 60,
    //         pathname: String::from("/folder/child"),
    //         name: String::from("child"),
    //         children: vec![],
    //     }));
    //     assert_eq!(metadata.id(), 50);
    //     assert_eq!(metadata.parent_id(), 60);
    //     assert_eq!(metadata.pathname(), String::from("/folder/child"));
    //     assert_eq!(metadata.name(), String::from("child"));
    //     assert_eq!(metadata.is_symlink(), false);
    //     assert_eq!(metadata.size(), 0);
    //     assert_eq!(metadata.created(), 0);
    //     assert_eq!(metadata.modified(), 0);
    //     assert_eq!(metadata.is_file(), false);
    //     assert_eq!(metadata.is_folder(), true);

    //     let metadata = super::Metadata::File(super::FileMd::new(&super::DbFile {
    //         id: 10,
    //         parent_id: 20,
    //         pathname: String::from("/foo/bar"),
    //         name: String::from("bar"),
    //         is_symlink: true,
    //         size: 1024,
    //         created: 780,
    //         modified: 1780,
    //     }));
    //     assert_eq!(metadata.id(), 10);
    //     assert_eq!(metadata.parent_id(), 20);
    //     assert_eq!(metadata.pathname(), String::from("/foo/bar"));
    //     assert_eq!(metadata.name(), String::from("bar"));
    //     assert_eq!(metadata.is_symlink(), true);
    //     assert_eq!(metadata.size(), 1024);
    //     assert_eq!(metadata.created(), 780);
    //     assert_eq!(metadata.modified(), 1780);
    //     assert_eq!(metadata.is_file(), false);
    //     assert_eq!(metadata.is_folder(), false);

    //     let metadata = super::Metadata::File(super::FileMd::new(&super::DbFile {
    //         id: 0,
    //         parent_id: 0,
    //         pathname: String::default(),
    //         name: String::default(),
    //         is_symlink: false,
    //         size: 0,
    //         created: 0,
    //         modified: 0,
    //     }));
    //     assert!(metadata.is_file());
    // }

    #[test]
    fn as_hierarchy() {
        // collect_fs_metadata(PathBuf::from(r"g:\testcase"), PathBuf::from("hierarchy.yaml"));
        let testcase_data = include_str!("hierarchy.yaml");
        let fs_metadata: filesys::FsMetadata = serde_yaml::from_str(testcase_data).unwrap();
        let conn = test_db_connection(Some(&fs_metadata));
        let mut hierarchy = super::Hierarchy::default();
        db::folder_tree_by_pathname_query(&conn, r"T:\testcase", |folder| -> Result<bool, db::Error> {
            hierarchy.add(folder);
            Ok(true)
        })
        .unwrap();
        hierarchy.flush();
        for md in hierarchy.get() {
            println!("{}", md);
        }
    }
}
