# Weather Data lib

The weather data library manages historical weather data.

### The `admin` module.

This module contains the administrative API `WeatherAdmin` and the entities specific administration.

### The `entities` module.

This module contains all the structures used to implement weather data commands.

### The `history_client` module.

This module contains the `HistoryClient` used to get historical weather data. The client
is built on top of the `reqwest` crate and contains the *Visual Crossing* `Rest` client 
implementation.

### The `weather_data` module.

This module contains the `WeatherData` API.

### The `backend` Module

The `backend` module implements the `DataAPI` used by the `WeatherData` API. It defines a 
`DataAdapter` trait used to implement the various weather data storage configurations.
This is arguably the most interesting within the library.

Regardless of the implementation historical weather data continues to be written into `Zip` 
archives and new locations are written into the locations document. This makes it easy to backup 
data and switch between implementations.

#### The `backend::filesys` module.

This module contains support for the files used in weather data. It implements `Zip` file
archive reading and update along with the weather locations `JSON` document. It also has 
operating system independent implementations for weather data directories and files.

#### The `backend::db` module.

This module contains the various database implementations available. All of the implementations 
are currently built on top of `Sqlite3`. There are three (3) implementations.

* A *hybrid* model where location information and weather data metadata is stored in the 
  database.  Weather history data history is stored in the `ZIP` archives.
* A *document* based model where all weather data history is stored in the database. Weather 
  history data is stored as `JSON` documents in the database. The `JSON` documents can 
  optionally be compressed.
* A fully *normalized* database model where all weather data history and locations are stored in 
  tables.
