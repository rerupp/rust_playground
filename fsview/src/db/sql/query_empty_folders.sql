SELECT
    folders.id AS folder_id,
    folders.parent_id AS folder_parent_id,
    folders.pathname AS folder_pathname
FROM
    files
    INNER JOIN
        folders on folders.id = files.parent_id
WHERE
    files.name = :empty_folder_filename
ORDER BY
    folder_pathname;