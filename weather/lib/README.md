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
archives and new locations are written into the locations document. This is a convenient way
to easily back data and reload the database as changes occur.

#### The `backend::filesys` module.

This module contains support for the files used in weather data. It implements `Zip` file
archive reading and writing along with the weather locations `JSON` document. It also has 
operating system independent implementations for weather data directories and files.

#### The `backend::db` module.

This module contains support for the database implementation of weather data history. It
also uses the `filesys` module to update the weather history archives and locations document
as changes are made.
