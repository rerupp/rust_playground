//! The location histories summary window.
use super::*;
use reports::list_history::text::Report;

/// The main tab window showing the location history dates that are available.
///
pub struct HistoriesWindow {
    /// Indicates the tab window is active or not.
    active: bool,
    /// The location history dates report view.
    report: Option<ReportView>,
    /// The weather data history API that will be used.
    weather_data: Rc<WeatherData>,
}
impl Debug for HistoriesWindow {
    /// Show all the attributes except the weather data API.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SummaryWindow").field("active", &self.active).field("report", &self.report).finish()
    }
}
impl HistoriesWindow {
    /// Create a new instance of the tab window.
    ///
    /// # Arguments
    ///
    /// - `weather_data` is the weather history API that will be used.
    ///
    pub fn new(weather_data: Rc<WeatherData>) -> Result<Self> {
        let mut fles = Self { active: false, report: None, weather_data };
        fles.refresh()?;
        Ok(fles)
    }
}
impl DialogWindow for HistoriesWindow {
    /// Query if the tab window is active or not.
    ///
    fn is_active(&self) -> bool {
        self.active
    }
    /// Control if the tab window is active or not.
    ///
    /// # Arguments
    ///
    /// - `yes_no` determines if the dialog is active or not.
    ///
    fn set_active(&mut self, yes_no: bool) {
        self.active = yes_no;
    }
    /// Force the tab to recreate the location history dates view.
    ///
    fn refresh(&mut self) -> std::result::Result<(), String> {
        self.report.take();
        match self.weather_data.get_history_dates(DataCriteria::default()) {
            Ok(history_dates) => {
                let report = Report::default().with_date_format("%b-%d-%Y").generate(history_dates);
                self.report.replace(ReportView::new(report, None).with_show_selected(true));
                Ok(())
            }
            Err(err) => Err(format!("Histories error ({})", err)),
        }
    }
    /// Get the size of the tab window.
    ///
    fn size(&self) -> Size {
        self.report.as_ref().map_or(Size::default(), |report| report.size())
    }
    /// Dispatch a key pressed event to the tab window. [ControlFlow::Continue] will be returned if the
    /// event is not consumed.
    ///
    /// # Arguments
    ///
    /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
    ///
    fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
        log_key_pressed!("HistoriesWindow");
        match self.report.take() {
            None => {
                debug_assert!(false, "key_pressed bad state\n{:#?}", self)
            }
            Some(mut report) => {
                // give the report a chance to eat the event
                let result = report.key_pressed(&key_event);
                self.report.replace(report);
                if let ControlFlow::Break(control_result) = result {
                    if ControlResult::NotAllowed == control_result {
                        beep();
                    }
                    break_event!(DialogResult::Continue)?
                }
            }
        }
        ControlFlow::Continue(())
    }
    /// Draw the tab window on the terminal screen and optionally return the current cursor position.
    ///
    /// # Arguments
    ///
    /// - `area` is where on the terminal screen the window will be drawn.
    /// - `buffer` is the current view of the terminal screen.
    ///
    fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
        log_render!("HistoriesWindow");
        self.report.as_ref().map_or(None, |report| {
            let styles = report.catalog_type.get_styles(ControlState::Active);
            report.render(area, buffer, styles)
        })
    }
}
