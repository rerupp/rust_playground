-- create an anchor that follows the folder tree
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
        folder_hierarchy.parent_id in ( SELECT folder_match.id FROM folders folder_match WHERE folder_match.name = :folder_name )
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

-- get the matching folder child files
SELECT
    hierarchy.id AS folder_id,
    hierarchy.parent_id AS folder_parent_id,
    hierarchy.pathname as folder_pathname,
    hierarchy.name as folder_name,
    hierarchy.size as folder_size,
    hierarchy.created as folder_created,
    hierarchy.modified as folder_modified,
    hierarchy_child.id AS file_id,
    hierarchy_child.parent_id AS file_parent_id,
    hierarchy_child.pathname AS file_pathname,
    hierarchy_child.name AS file_name,
    hierarchy_child.is_symlink as file_is_symlink,
    hierarchy_child.size AS file_size,
    hierarchy_child.created AS file_created,
    hierarchy_child.modified AS file_modified
FROM
    hierarchy
    INNER JOIN
        files hierarchy_child ON hierarchy_child.parent_id = hierarchy.id

-- get the matching folder files
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
    parent.id IN ( SELECT child_parent.id FROM folders child_parent WHERE child_parent.name = :folder_name )

ORDER BY
    folder_pathname, file_pathname;