BEGIN;

-- The mode the database is running in
CREATE TABLE IF NOT EXISTS config
(
    id INTEGER PRIMARY KEY,
    hybrid INTEGER NOT NULL,
    document INTEGER NOT NULL,
    full INTEGER NOT NULL,
    compress INTEGER NOT NULL
);

-- The weather locations table
CREATE TABLE IF NOT EXISTS locations
(
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    alias TEXT NOT NULL,
    longitude TEXT NOT NULL,
    latitude TEXT NOT NULL,
    tz TEXT NOT NULL
);

-- cover the location name with an index
CREATE UNIQUE INDEX IF NOT EXISTS idx_location_name ON locations(name);

-- cover the location alias with an index
CREATE UNIQUE INDEX IF NOT EXISTS idx_location_alias ON locations(alias);

-- create the location metadata table
CREATE TABLE if NOT EXISTS metadata
(
    id INTEGER PRIMARY KEY,
    lid INTEGER NOT NULL,
    date TEXT NOT NULL,
    store_size INTEGER,
    size INTEGER,
    mtime INTEGER,
    FOREIGN KEY (lid) REFERENCES locations(id)
);

-- cover the metadata location id with an index
CREATE INDEX IF NOT EXISTS idx_dates_lid on metadata(lid);

-- cover the metadata dates with an index
CREATE INDEX IF NOT EXISTS idx_metadata_date on metadata(date);

-- create the document based history table
CREATE TABLE IF NOT EXISTS documents
(
    id INTEGER PRIMARY KEY,
    mid INTEGER NOT NULL,
    daily TEXT,
    daily_zip BLOB,
    -- the size of the uncompressed daily history
    daily_size INTEGER,
    FOREIGN KEY (mid) REFERENCES metadata(id)
);

-- cover the metadata id with an index
CREATE INDEX IF NOT EXISTS idx_documents_mid on documents(mid);

-- create the daily history table
CREATE TABLE IF NOT EXISTS daily
(
    id INTEGER PRIMARY KEY,
    mid INTEGER NOT NULL,
    temp_high REAL,
    temp_high_t INTEGER,
    temp_low REAL,
    temp_low_t INTEGER,
    temp_max REAL,
    temp_max_t INTEGER,
    temp_min REAL,
    temp_min_t INTEGER,
    wind_speed REAL,
    wind_gust REAL,
    wind_gust_t INTEGER,
    wind_bearing INTEGER,
    cloud_cover REAL,
    uv_index INTEGER,
    uv_index_t INTEGER,
    summary TEXT,
    humidity REAL,
    dew_point REAL,
    sunrise_t INTEGER,
    sunset_t INTEGER,
    moon_phase REAL,
    FOREIGN KEY (mid) REFERENCES metadata(id)
);

-- cover the metadata id with an index
CREATE INDEX IF NOT EXISTS idx_daily_mid on daily(mid);

COMMIT;
