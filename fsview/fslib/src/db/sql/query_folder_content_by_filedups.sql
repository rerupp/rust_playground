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
    parent.id in ( SELECT DISTINCT(filedups.parent_id) FROM filedups )
ORDER BY
    folder_pathname, file_pathname;
