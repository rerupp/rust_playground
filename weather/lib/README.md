# Weather Data lib

The weather data library contains the implementation of accessing and updating historical weather data. The library defines the API used by the *weather* and the *admin* binaries.

 The library consists of the following top-level objects and modules.

* The library `Result` and `Error` returned from from the API calls.
* The `api` module defines the `WeatherData` API for clients such as the *weather* binary.
* The `admin_api` module defines `AdminApi` API for clients like the *admin* binary.
* The `entities` module contains the objects used by the API.
* The `backend` module contains the weather data implemenations.

## The backend Module

The `backend` module is probably the most interesting within the library. It contains the different weather data implementations that are available. At the module level a `DataAdapter` trait is defined which is used by [WeatherData] to interact with weather data. There are 2 primary types of data adapter implementations.

* A `Zip` archive implementation in the `filesys` module.
* Several different database implementations in the `db` module.

### visual_crossing Module

The `visual_crossing` module has the `Timeline` client implementation. It isolates the accessing history weather data from the implementation of the backend store. The `Timeline` API key can be specified in the `weather.toml` file or through the process environment.

### filesys Module

The `filesys` module follows the original `Python` implementation. Weather data is stored within some filesystem directory. Location information is stored in a `JSON` document and historical weather data is stored in location specific `Zip` archives as `JSON` documents.

A brief description of the sub-modules follows.

* `file` contains the file system models for directories and files within directories.
* `archive` contains the `ZIP` models to read and write weather data archives.
* `locations` contains the models to read and write location information.
* `adapter` contains the implementation of the data adapter trait.

It bugs me the `file` module is burried within the `filesys` module. Both the `WeatherDir` and `WeatherFile` are used by the `db` implementations. It doesn't seem right the `db` would be dependent on `filesys` but it is. Maybe at some point I'll move `file` out but for now the dependency remains.

### Database implementation

The database implementations are built on top of `Sqlite3`. There are three (3) implementations of weather data.

* A *hybrid* model where location information and weather data metadata is stored in the database. Weather history data is read from `ZIP` archives.
* A *document* based model where all weather data is stored in the database. Weather history data is stored as `JSON` documents in the database. The `JSON` documents can optionally be compressed.
* A *normalized* data model.

A brief description of the sub-modules follows.

* `admin` contains the `API` used to initialize and load the database.
* `archive_loader` contains a thread based database loader used by the `documents` and `normalized` module. It uses the *Rust* `mpsc` module to separate archive data mining from database updates.
* `query` contains the database queries that are not specific to a model's implementation.
* `locations` contains the models to read and write location data.
* `hybrid` contains the *hybrid* model implementation.
* `document` contains the *document* model implementation.
* `normalized` contains the *normalized* model implementation.
