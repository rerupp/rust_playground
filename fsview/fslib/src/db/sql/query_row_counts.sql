SELECT
    (SELECT COUNT(*) FROM folders) AS total_folders,
    (SELECT COUNT(*) FROM files) AS total_files,
    (SELECT COUNT(*) FROM problems) AS total_problems
