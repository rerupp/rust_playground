# Weather Data lib

The weather data library that provides the backend domain implementation for weather data.

### The `domain` Module

The `domain` module differs from the `Python` implementation in that weather data is accessed through an API defined in the `data` module. The domain API is light delegating most of the work to the `data` module. The domain defines the bean definitions used to pass weather data between modules.

### The `data` Module

The `data` module is designed to support multiple persistent implementations. The `DataAPI` trait defines the API used by the domain to access weather data. The data module defines some public beans used to query weather data otherwise it uses beans defined in the `domain` module.

Currently there is only one (1) implementor of the `DataAPI` based on ZIP file archives. At some point I'll consider using a hybrid approach using Sqlite3 and ZIP file archives.

#### The *`fs`* Module

The `fs` module contains a filesystem implementation of the `DataAPI`.

Similar to the `Python` implementation, weather data is stored in the filesystem in a directory. By default the directory is named `weather_data` but that can be changed using the `--directory` argument or `WEATHER_DATA` environment variable.

The following table describes the expected files within the folder.

| File | Description |
| :--- | :--- |
| `locations.json` | The cities (or locations) that have been defined |
| `{location-alias}.zip` | The archive containing weather data for the location |

Given the following weather data locations shown by the `ll` command.

```
$ weather.exe ll
      Location            Alias          Longitude/Latitude           Timezone
--------------------- -------------- --------------------------- -------------------
Boise, ID             boise_id           -116.2312/43.6007       America/Boise
Carson City, NV       carson_city_nv     -119.7474/39.1511       America/Los_Angeles
Fortuna Foothills, AZ foothills       -114.4118901/32.6578355    America/Phoenix
Indio, CA             indio           -116.2188054/33.7192808    America/Los_Angeles
Klamath Falls, OR     kfalls             -121.7754/42.2191       America/Los_Angeles
Lake Havasu City, AZ  havasu          -114.3224495/34.4838502    America/Phoenix
Lake Oswego, OR       lake_oswego_or     -122.7003/45.4129       America/Los_Angeles
Las Cruces, NM        las_cruces_nm      -106.7893/32.3265       America/Denver
Las Vegas, NV         vegas           -115.1485163/36.1672559    America/Los_Angeles
Medford, OR           medford            -122.8537/42.3372       America/Los_Angeles
Mesa, AZ              mesa            -111.8314773/33.4151117    America/Phoenix
Roseburg, OR          roseburg           -123.3518/43.2232       America/Los_Angeles
Seattle, WA           seattle         -122.3300624/47.6038321    America/Los_Angeles
St. George, UT        stgeorge        -113.5841313/37.104153     America/Denver
Tigard, OR            tigard             -122.7845/45.4237       America/Los_Angeles
Tucson, AZ            tucson          -110.9748477/32.2228765    America/Phoenix
```

The `weather_data` folder would have the following content.

```

$ dir weather_data
 Volume in drive G is DEV
 Volume Serial Number is 6631-8343

 Directory of G:\dev\weather_data

07/10/2020  01:28 PM    <DIR>          .
07/10/2020  01:28 PM    <DIR>          ..
07/10/2020  08:54 AM         2,035,697 boise_id.zip
07/10/2020  08:30 AM         2,093,135 carson_city_nv.zip
06/01/2020  12:39 PM         2,044,268 foothills.zip
06/28/2020  12:34 PM         1,977,714 havasu.zip
06/28/2020  12:32 PM         2,034,949 indio.zip
07/08/2020  09:25 AM         2,029,631 kfalls.zip
06/01/2020  12:39 PM            56,062 lake_oswego_or.zip
06/01/2020  12:39 PM         1,592,860 las_cruces_nm.zip
07/04/2020  09:05 AM             2,693 locations.json
07/08/2020  09:20 AM         2,004,853 medford.zip
06/01/2020  12:39 PM         2,038,098 mesa.zip
07/07/2020  08:58 AM         1,980,186 roseburg.zip
06/01/2020  12:39 PM           145,678 seattle.zip
07/10/2020  01:28 PM         1,680,247 stgeorge.zip
07/07/2020  08:56 AM         2,065,225 tigard.zip
06/01/2020  12:39 PM         2,035,792 tucson.zip
06/01/2020  12:39 PM            53,839 vegas.zip
              17 File(s)     25,870,927 bytes
               2 Dir(s)  498,958,962,688 bytes free
```

Details about the weather data storage can be viewed using the `ls` command.

```
$ weather.exe ls
      Location        Overall Size History Count Raw History Size Compressed Size
--------------------- ------------ ------------- ---------------- ---------------
Boise, ID                1,988 Kib         1,284       11,543 Kib       1,815 Kib
Carson City, NV          2,044 Kib         1,284       11,640 Kib       1,841 Kib
Fortuna Foothills, AZ    1,996 Kib         1,274       13,839 Kib       1,820 Kib
Indio, CA                1,987 Kib         1,274       13,945 Kib       1,830 Kib
Klamath Falls, OR        1,982 Kib         1,284       11,557 Kib       1,819 Kib
Lake Havasu City, AZ     1,931 Kib         1,274       13,923 Kib       1,770 Kib
Lake Oswego, OR             55 Kib            31          308 Kib          50 Kib
Las Cruces, NM           1,556 Kib         1,061        9,204 Kib       1,392 Kib
Las Vegas, NV               53 Kib            31          304 Kib          49 Kib
Medford, OR              1,958 Kib         1,284       11,398 Kib       1,790 Kib
Mesa, AZ                 1,990 Kib         1,274       13,975 Kib       1,839 Kib
Roseburg, OR             1,934 Kib         1,284       11,445 Kib       1,761 Kib
Seattle, WA                142 Kib            92          850 Kib         130 Kib
St. George, UT           1,641 Kib         1,061       11,403 Kib       1,498 Kib
Tigard, OR               2,017 Kib         1,284       11,870 Kib       1,854 Kib
Tucson, AZ               1,988 Kib         1,274       14,004 Kib       1,826 Kib
===================== ============ ============= ================ ===============
Total                   25,262 Kib        16,350      161,208 Kib      23,082 Kib
```
