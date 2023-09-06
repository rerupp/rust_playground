# Weather Data lib

The library that implements weather data in `Rust`. The library consists of the following top-level objects and modules.

* The library `Error` that can be returned from calls to the library.
* The `api` module defines the `API` clients use to access weather data.
* The `admin_api` module defines the `API` clients use to administer weather data.
* The `entities` module contains the objects used by the `API`s.
* The `backend` module contains the weather data implemenations.

## The backend Module

This module contains the weather data implementation. It defines a `DataAdapter` trait the weather data `API` uses as the implementation. There are currently two (2) main implementations of the data adapter.

* A `Zip` archive implementation in the `filesys` module.
* A database implementation in the `db` module.

### Zip Archive Implementation

This implementation follows the original `Python` implementation where weather data consists of `JSON` documents stored in files and `ZIP` archives.

#### Inner module descriptions.

* `file` contains the file system models for directories and files within  directories.
* `archive` contains the `ZIP` models to read and write weather data archives.
* `locations` contains the models to read and write location information.
* `adapter` contains the implementation of the data adapter trait.

### Database implementation

The database implementation is built on top of `Sqlite3`. There are three (3) implementations of weather data.

* A *hybrid* model where location information and weather data metadata is stored in the database. Weather history data is read from `ZIP` archives.
* A *document* based model where all weather data is stored in the database. Weather history data is stored as `JSON` documents in the database. The `JSON` documents can optionally be compressed.
* A *normalized* data model.

#### Inner module description.

* `admin` contains the `API` used to initialize and load the database.
* `archive_loader` contains a thread based database loader used by the `documents` and `normalized` module. It uses the *Rust* `mpsc` module to separate archive data mining from database updates.
* `query` contains the database queries that are not specific to a model's implementation.
* `locations` contains the models to read and write location data.
* `hybrid` contains the *hybrid* model implementation.
* `document` contains the *document* model implementation.
* `normalized` contains the *normalized* model implementation.
