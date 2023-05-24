SELECT
    filedups.parent_id AS folder_id,
    folders.parent_id AS folder_parent_id,
    folders.pathname AS folder_pathname,
    -- folders.name AS folder_name,
    -- folders.size AS folder_size,
    -- folders.created AS folder_created,
    -- folders.modified AS folder_modified,
    filedups.file_id AS file_id,
    files.parent_id AS file_parent_id,
    files.pathname AS file_pathname,
    files.name AS file_name,
    -- files.is_symlink AS file_is_symlink,
    files.size AS file_size
    -- files.created AS file_created,
    -- files.modified AS file_modified
FROM
    filedups
JOIN
    folders ON filedups.parent_id = folders.id
JOIN
    files ON filedups.file_id = files.id
ORDER BY
    file_name;
