from .weather_data import (WeatherData, DailyHistories, DateRange, HistoryClient, LocationHistoryDates,
                           HistorySummaries, Location, LocationCriteria, LocationFilter, LocationFilters)


class WeatherConfigException(Exception):
    def __init__(self, message: str):
        self.add_note(message)

    def reason(self):
        '\n'.join(self.__notes__)


__all__ = ['WeatherData', 'WeatherConfigException', 'DailyHistories', 'DateRange', 'HistoryClient',
           'LocationHistoryDates', 'HistorySummaries', 'Location', 'LocationCriteria', 'LocationFilter',
           'LocationFilters']
