SELECT
    fldrs.id AS folder_id,
    fldrs.parent_id AS folder_parent_id,
    fldrs.pathname AS folder_pathname,
    fldrs.name as folder_name
FROM
    folders fldrs
JOIN
(
    SELECT
        name, COUNT(*)
    FROM
        folders 
    GROUP BY
        name
    HAVING
        COUNT(*) > 1
) AS dups
ON
    fldrs.name = dups.name
ORDER BY
    -- fldrs.pathname, fldrs.name
    fldrs.name
    