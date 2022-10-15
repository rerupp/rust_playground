-- the target subdirectories
SELECT
    child_folder.id AS folder_id,
    child_folder.parent_id AS folder_parent_id,
    child_folder.pathname AS folder_pathname,
    child_folder.name AS folder_name,
    child_folder.size AS folder_size,
    child_folder.created AS folder_created,
    child_folder.modified AS folder_modified,
    -1 AS file_id,
    -1 AS file_parent_id,
    "" AS file_pathname,
    "" AS file_name,
    0 AS file_is_symlink,
    0 AS file_size,
    0 AS file_created,
    0 AS file_modified
FROM
    folders child_folder
WHERE
    child_folder.parent_id in ( SELECT parent_folder.id FROM folders parent_folder WHERE parent_folder.name = :folder_name )
-- the target folder children
UNION ALL
SELECT
    parent.id AS folder_id,
    parent.parent_id AS folder_parent_id,
    parent.pathname AS folder_pathname,
    parent.name AS folder_name,
    parent.size AS folder_size,
    parent.created AS folder_created,
    parent.modified AS folder_modified,
    child.id AS file_id,
    child.parent_id AS file_parent_id,
    child.pathname AS file_pathname,
    child.name AS file_name,
    child.is_symlink AS file_is_symlink,
    child.size AS file_size,
    child.created AS file_created,
    child.modified AS file_modified
FROM
    folders parent
    INNER JOIN
        files child ON child.parent_id = parent.id
WHERE
    parent.id in ( SELECT child_parent.id FROM folders child_parent WHERE child_parent.name = :folder_name )
ORDER BY
    folder_pathname, file_pathname;
