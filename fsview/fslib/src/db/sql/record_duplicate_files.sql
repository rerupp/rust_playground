-- use a transaction to mask the table chnages
BEGIN;

-- clean the table contents
DELETE from filedups;

-- find all the dupicate filenames and insert them into the table
INSERT INTO filedups (file_id, parent_id)
SELECT
    id AS file_id,
    parent_id
FROM
    files
JOIN
(
    SELECT
        name, COUNT(*)
    FROM
        files
    WHERE
        name <> '<?>'
    GROUP BY
        name
    HAVING
        COUNT(*) > 1
) AS dups
ON
    files.name = dups.name
ORDER BY
    file_id;

-- make the table changes visible
COMMIT;
    