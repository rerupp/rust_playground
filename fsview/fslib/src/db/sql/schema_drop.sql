-- remove tables that have a foreign key with files and folders
DROP TABLE IF EXISTS dupfiles;
-- remove tables that have a foreign key with folders
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS problems;
-- now folders can be removed
DROP TABLE IF EXISTS folders;
