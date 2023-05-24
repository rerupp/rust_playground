BEGIN;

-- folders in a filesystem
CREATE TABLE IF NOT EXISTS folders
(
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    pathname TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    created INTEGER NOT NULL,
    modified INTEGER NOT NULL
);

-- cover the folder pathname with an index
CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_pathname ON folders(pathname);

-- also cover the folder name with an index
CREATE INDEX IF NOT EXISTS idx_folders_name ON folders(name);

-- metadata for files in a filesystem
CREATE TABLE IF NOT EXISTS files
(
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    pathname TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    is_symlink INTEGER NOT NULL,
    size INTEGER NOT NULL,
    created INTEGER NOT NULL,
    modified INTEGER NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES folders(id)
);

-- cover the file pathname with an index
CREATE UNIQUE INDEX IF NOT EXISTS idx_files_pathname ON files(pathname);

-- cover the name of the file with an index
CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);

-- problems encountered loading files and folders
CREATE TABLE IF NOT EXISTS problems
(
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    pathname TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES folders(id)
);

-- cover the problem pathname with an index
CREATE UNIQUE INDEX IF NOT EXISTS idx_problems_pathname ON problems(pathname);

-- create the duplicate filenames table
CREATE TABLE IF NOT EXISTS filedups
(
    file_id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES files(id),
    FOREIGN KEY (parent_id) REFERENCES folders(id)
) WITHOUT ROWID;

-- cover the associated folder with an index
CREATE INDEX IF NOT EXISTS idx_dupfiles_parent ON filedups(parent_id);

COMMIT;