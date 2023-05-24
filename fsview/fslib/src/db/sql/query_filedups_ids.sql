SELECT
    files.name as filename,
    filedups.file_id as file_id,
    filedups.parent_id as parent_id
FROM
    filedups
JOIN
    files ON filedups.file_id = files.id
JOIN
    folders ON filedups.parent_id = folders.id
ORDER BY
    filename;