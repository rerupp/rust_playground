# `fslib` Package

The `fsview` package produces the library used by the mainline. It consist of three (3) modules.

* The `domain` module is what it sounds like, the fsview domain objects and API. The cli uses the domains `Session` object to load and access file system metadata.
* The `filesys` module contains the internal API used by the `domain` to collect local filesystem metadata.
* The `db` module contains the internal API used by the `domain` to save and query the collected filesystem metadata.

#### `domain` Module

The `domain` module contains the public API and objects that front end access to file system information. It is the middle layer between client and data persistence.

The module contains the following submodules.

* `api` contains the implementation of the `session` api.
* `filedups` contains the objects and functions used for duplicate filenames.
* `objects` contains the public facing structures used by the `session`.

Most of the heavy lifting is loading a folders metadata and converting the results of a query into hierarchical folder structure.

#### `filesys` Module

The `filesys` module contains the internal API that traverses a directory collecting metadata. It abstacts the file system `stat` information into various metadata containers. The domain uses the metadata objects to load the database with information for the folder hierarchy.

#### `db` Module

Originally I thought the tool could be stateless however I soon realized that was not going to work. It can take upwards to 20 minutes to load the 500k+ folders and 2.6M files across the five (5) disks I have.

I was not looking forward to requiring some database like Postgres or Mongo to be installed in order to have quicker access to the metadata. Imagine my delight when I discovered there is a SQLite crate available, `rusqlite`, that does not require an external server to be installed.

SQL such as the schema initialization and queries are externalized into separate files contained in a separate folder. The code uses the standard `include_str!` macro to load them into string constants at compile time. The Rust documentation states the provided path is platform specific however the `include_str!` macros use a Linux relative path and it is currently working on Windoz.

The SQL database engine is reasonably fast and so far it has been reliable. The database file itself requires about 1.5 Gib of disk space after loading the metadata from my system. I suspect performance of Postgres or Mongo would be better due to caching on the server however when you consider the CLI database access is essentially stateless, performance right now is good enough.

Well kind of. When you ask for something like `fsview list --name src -SR` across the 2.6M files it can take over a minute to complete. I'm not a SQL engineer so I'm sure there is a better way to query but when you consider it contains ~120k folder summaries I think it should hurt a bit.
