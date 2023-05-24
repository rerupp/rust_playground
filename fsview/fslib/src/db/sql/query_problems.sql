SELECT
    folders.id AS folder_id,
    folders.parent_id AS folder_parent_id,
    folders.pathname AS folder_pathname,
    problems.id AS problem_id,
    problems.parent_id AS problem_parent_id,
    problems.pathname AS problem_pathname,
    problems.description AS problem_description
FROM
    folders
    INNER JOIN
        problems on problems.parent_id = folders.id
ORDER BY
    folder_pathname, problem_pathname;
