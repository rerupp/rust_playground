SELECT
    root_folder.id AS folder_id,
    root_folder.parent_id AS folder_parent_id,
    root_folder.pathname AS folder_pathname,
    root_folder.name AS folder_name,
    root_folder.size AS folder_size,
    root_folder.created AS folder_created,
    root_folder.modified AS folder_modified,
    -1 AS file_id,
    -1 AS file_parent_id,
    "" AS file_pathname,
    "<?>" AS file_name,
    0 AS file_is_symlink,
    0 AS file_size,
    0 AS file_created,
    0 AS file_modified
FROM
    folders root_folder
WHERE
    root_folder.parent_id in ( SELECT mf.id FROM folders mf WHERE mf.parent_id = :parent_id )
-- the target folder children
UNION ALL
SELECT
    root_file_folder.id AS folder_id,
    root_file_folder.parent_id AS folder_parent_id,
    root_file_folder.pathname AS folder_pathname,
    root_file_folder.name AS folder_name,
    root_file_folder.size AS folder_size,
    root_file_folder.created AS folder_created,
    root_file_folder.modified AS folder_modified,
    root_file.id AS file_id,
    root_file.parent_id AS file_parent_id,
    root_file.pathname AS file_pathname,
    root_file.name AS file_name,
    root_file.is_symlink AS file_is_symlink,
    root_file.size AS file_size,
    root_file.created AS file_created,
    root_file.modified AS file_modified
FROM
    folders root_file_folder
    INNER JOIN
        files root_file ON root_file.parent_id = root_file_folder.id
WHERE
    root_file_folder.id in ( SELECT mf.id FROM folders mf WHERE mf.parent_id = :parent_id )
ORDER BY
    folder_pathname, file_pathname;
