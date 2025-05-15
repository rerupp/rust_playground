from typing import List

import py_weather_lib as wd
from py_weather_lib import (DailyHistories, DataCriteria, DateRange, HistoryClient, HistorySummaries, Location,
                            LocationCriteria, LocationHistoryDates)

__all__ = ['WeatherData', 'DailyHistories', 'DataCriteria', 'DateRange', 'HistoryClient', 'LocationHistoryDates',
           'HistorySummaries', 'Location', 'LocationCriteria']


class WeatherData:
    """
    Plumbing signatures through pyo3 is a PITA right now due to it requiring .pyi files.
    Since I'm the sole consumer of the Rust bindings this is easier to maintain than
    interface files.
    """

    def __init__(self, rust_bindings: wd.WeatherData = None):
        self._rust_bindings = rust_bindings

    def add_histories(self, daily_histories: DailyHistories) -> int:
        return self._rust_bindings.add_histories(daily_histories)

    def get_history_client(self) -> HistoryClient:
        return self._rust_bindings.get_history_client()

    def get_daily_history(self, criteria: DataCriteria, history_range: DateRange) -> DailyHistories:
        return self._rust_bindings.get_daily_history(criteria, history_range)

    def get_history_dates(self, criteria=DataCriteria()) -> List[LocationHistoryDates]:
        return self._rust_bindings.get_history_dates(criteria)

    def get_history_summary(self, criteria=DataCriteria()) -> List[HistorySummaries]:
        return self._rust_bindings.get_history_summary(criteria)

    def get_locations(self, criteria=DataCriteria()) -> List[Location]:
        return self._rust_bindings.get_locations(criteria)

    def search_locations(self, criteria: LocationCriteria) -> List[Location]:
        return self._rust_bindings.search_locations(criteria)

    def add_location(self, location: Location):
        self._rust_bindings.add_location(location)
