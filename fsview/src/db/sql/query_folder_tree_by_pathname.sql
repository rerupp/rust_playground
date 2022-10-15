-- create an anchor that follows the folder hierarchy
WITH hierarchy AS
(
    SELECT
        folder_hierarchy.id,
        folder_hierarchy.parent_id,
        folder_hierarchy.pathname,
        folder_hierarchy.name,
        folder_hierarchy.size,
        folder_hierarchy.created,
        folder_hierarchy.modified
    FROM
        folders folder_hierarchy
    WHERE
        folder_hierarchy.pathname = :folder_pathname
    UNION
    SELECT
        sub_folder.id,
        sub_folder.parent_id,
        sub_folder.pathname,
        sub_folder.name,
        sub_folder.size,
        sub_folder.created,
        sub_folder.modified
    FROM
        folders sub_folder
        INNER JOIN
            hierarchy ON hierarchy.id = sub_folder.parent_id
)
-- get the children folders and files from the hierarchy
SELECT
    hierarchy.id AS folder_id,
    hierarchy.parent_id AS folder_parent_id,
    hierarchy.pathname as folder_pathname,
    hierarchy.name as folder_name,
    hierarchy.size AS folder_size,
    hierarchy.created AS folder_created,
    hierarchy.modified AS folder_modified,
    hierarchy_files.id AS file_id,
    hierarchy_files.parent_id AS file_parent_id,
    hierarchy_files.pathname AS file_pathname,
    hierarchy_files.name AS file_name,
    hierarchy_files.is_symlink as file_is_symlink,
    hierarchy_files.size AS file_size,
    hierarchy_files.created AS file_created,
    hierarchy_files.modified AS file_modified
FROM
    hierarchy
    INNER JOIN
        files hierarchy_files ON hierarchy_files.parent_id = hierarchy.id
ORDER BY
    folder_pathname, file_pathname;