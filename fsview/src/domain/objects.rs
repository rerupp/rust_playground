use std::fmt::Display;
use std::fmt::Write as FmtWrite;

/// The types of metadata that can be retrieved from the database.
#[derive(Debug)]
pub enum Metadata {
    /// The top-level directory of a folder hierarchy.
    Root(FolderMd),
    /// A folders metadata.
    Folder(FolderMd),
    /// A files metadata.
    File(FileMd),
    /// Problems encountered when adding folders to the database.
    Problem(ProblemMd),
}

impl Metadata {
    /// Get the metadata unique identifier.
    pub fn id(&self) -> i64 {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => folder_md.id,
            Metadata::File(file_md) => file_md.id,
            Metadata::Problem(problem_md) => problem_md.id,
        }
    }
    /// Get the metadata parent identifier.
    pub fn parent_id(&self) -> i64 {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => folder_md.parent_id,
            // Metadata::Folder(folder_md) => folder_md.parent_id,
            Metadata::File(file_md) => file_md.parent_id,
            Metadata::Problem(problem_md) => problem_md.parent_id,
        }
    }
    /// Get the metadata pathname.
    pub fn pathname(&self) -> &str {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => &folder_md.pathname,
            // Metadata::Folder(folder_md) => &folder_md.pathname,
            Metadata::File(file_md) => &file_md.pathname,
            Metadata::Problem(problem_md) => &problem_md.pathname,
        }
    }
    /// get the metadata filename.
    pub fn name(&self) -> &str {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => &folder_md.name,
            // Metadata::Folder(folder_md) => &folder_md.name,
            Metadata::File(file_md) => &file_md.name,
            Metadata::Problem(problem_md) => &problem_md.name,
        }
    }
    /// Returns `true` if the metadata is a root variant.
    pub fn is_root(&self) -> bool {
        match self {
            Metadata::Root(_) => true,
            _ => false,
        }
    }
    /// Returns `true` if the metadata is a folder variant.
    pub fn is_folder(&self) -> bool {
        match self {
            Metadata::Folder(_) => true,
            _ => false,
        }
    }
    /// Returns `true` if the metadata is a file variant and is *not* a symbolic link.
    pub fn is_file(&self) -> bool {
        match self {
            Metadata::File(file_md) => !file_md.is_symlink,
            _ => false,
        }
    }
    /// Returns `true` if the metadata is a file variant and is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        match self {
            Metadata::File(file_md) => file_md.is_symlink,
            _ => false,
        }
    }
    /// Returns `true` if the metadata is a problem variant.
    pub fn is_problem(&self) -> bool {
        match self {
            Metadata::Problem(_) => true,
            _ => false,
        }
    }
    /// Returns the disk size of the variant.
    pub fn size(&self) -> u64 {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => folder_md.size,
            Metadata::File(file_md) => file_md.size,
            Metadata::Problem(_) => 0,
        }
    }
    /// Return the timestamp of when the metadata variant was created.
    pub fn created(&self) -> u64 {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => folder_md.created,
            // Metadata::Folder(folder_md) => folder_md.created,
            Metadata::File(file_md) => file_md.created,
            Metadata::Problem(_) => 0,
        }
    }
    /// Return the timestamp of when the metadata variant was last modified.
    pub fn modified(&self) -> u64 {
        match self {
            Metadata::Root(folder_md) | Metadata::Folder(folder_md) => folder_md.modified,
            // Metadata::Folder(folder_md) => folder_md.modified,
            Metadata::File(file_md) => file_md.modified,
            Metadata::Problem(_) => 0,
        }
    }
}
/// Display information about the metadata.
/// 
/// This can be an expensive operation if the folder hierarchy is deep.
impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn traverse(metadata: &Metadata, depth: usize) -> String {
            let mut content = String::default();
            let preamble = "| ".repeat(depth);
            if let Metadata::Root(folder_md) | Metadata::Folder(folder_md) = metadata {
                writeln!(content, "{}{}", preamble, metadata.pathname()).unwrap();
                let preamble = "| ".repeat(depth + 1);
                for metadata in &folder_md.children {
                    let file_type = match metadata {
                        Metadata::File(file_md) => if file_md.is_symlink {"SYML"} else {"FILE"},
                        Metadata::Root(_) => "ROOT",
                        Metadata::Folder(_) => "FLDR",
                        Metadata::Problem(_) => "PROB",
                    };
                    writeln!(content, "{}{} {}", preamble, file_type, metadata.name()).unwrap();
                }
                for metadata in &folder_md.children {
                    if metadata.is_folder() {
                        let child_content = traverse(metadata, depth + 1);
                        content.push_str(&child_content);
                    }
                }
            } else {
                writeln!(content, "{}Expected folder got '{}'", preamble, metadata.pathname()).unwrap();
            }
            content
        }
        write!(f, "{}", traverse(self, 0))
    }
}
/// The metadata associated with a folder.
#[derive(Debug)]
pub struct FolderMd {
    /// The folder unique identified.
    pub id: i64,
    /// The folders parent unique identifier.
    pub parent_id: i64,
    /// The pathname of the folder.
    pub pathname: String,
    /// The filename of the folder.
    pub name: String,
    /// The disk space consummed by the folder.
    pub size: u64,
    /// The timestamp of when the folder was created.
    pub created: u64,
    /// The timestamp of when the folder was last modified.
    pub modified: u64,
    /// The contents of the folder.
    pub children: Vec<Metadata>,
}
/// Create an empty instance of the folder metadata.
impl Default for FolderMd {
    fn default() -> Self {
        Self {
            id: -1,
            parent_id: -1,
            pathname: Default::default(),
            name: Default::default(),
            size: Default::default(),
            created: Default::default(),
            modified: Default::default(),
            children: Default::default()
         }
    }
}
/// Return `true` if the folder metadata is default.
impl FolderMd {
    pub fn is_default(&self) -> bool {
        self.id == -1 && self.parent_id == -1
    }
}

/// The metadata associated with a file.
#[derive(Debug)]
pub struct FileMd {
    /// The unique identifier of the file.
    pub id: i64,
    /// The file parent identifier.
    pub parent_id: i64,
    /// The pathname of the file.
    pub pathname: String,
    /// The filename.
    pub name: String,
    /// When `true` indicates the file is a symbolic link.
    pub is_symlink: bool,
    /// The disk space used by the file.
    pub size: u64,
    /// The timestamp of when the file was created.
    pub created: u64,
    /// The timestamp of when the file was last modified.
    pub modified: u64,
}
/// The metadata associated with a problem.
#[derive(Debug)]
pub struct ProblemMd {
    /// The unique identifier of the problem.
    pub id: i64,
    /// The problems parent identifier.
    pub parent_id: i64,
    /// The pathname of the problem.
    pub pathname: String,
    /// The problem filename.
    pub name: String,
    /// A description of the problem.
    pub description: String,
}
/// The database information metadata.
#[derive(Debug)]
pub struct DbInformation {
    /// The top-level folder pathnames.
    pub root_folders: Vec<String>,
    /// The total count of files that have been added.
    pub file_count: u64,
    /// The total count of folders that have been added.
    pub folder_count: u64,
    /// The total count of problems that were encountered.
    pub problem_count: u64,
    /// The database allocation size.
    pub database_size: u64,
}
