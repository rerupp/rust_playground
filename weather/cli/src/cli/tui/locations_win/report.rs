//! The latest version of report history
use super::*;
use criteria::CriteriaDialog;
use reports::report_history::{text::Report, ReportSelector};
use termui_lib::break_event;
use weather_lib::prelude::DateRange;

/// The criteria button identifier.
///
const CRITERIA_ID: &'static str = "CRITERIA";

/// The exit button identifier.
///
const EXIT_ID: &'static str = "EXIT";

/// The dialog that shows a locations history report.
///
pub struct ReportDialog {
    /// The report history dialog and window.
    dialog: ButtonDialog<ReportWindow>,
    /// The dialog that allows selection of history dates and category selection.
    criteria: CriteriaDialog,
    /// The name of the location.
    location_name: String,
    /// the location alias name.
    location_alias: String,
    /// The weather data history API that will be used.
    weather_data: Rc<WeatherData>,
}
impl Debug for ReportDialog {
    /// Show all the attributes except the weather data API.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReportDialog")
            .field("dialog", &self.dialog)
            .field("criteria", &self.criteria)
            .field("location_name", &self.location_name)
            .field("location_alias", &self.location_alias)
            .finish()
    }
}
impl ReportDialog {
    /// Create a new instance of the report dialog.
    ///
    /// # Arguments
    ///
    /// - `location` allows the name and alias to be mined.
    /// - `weather_data` is the weather history API that will be used.
    ///
    pub fn new(location: &Location, weather_data: Rc<WeatherData>) -> Self {
        Self {
            dialog: ButtonDialog::new(
                ButtonBar::new(vec![
                    Button::new(CRITERIA_ID, "Criteria", 'C').with_active(),
                    Button::new(EXIT_ID, "Exit", 'x'),
                ])
                .with_auto_select(true),
                ReportWindow::default(),
            )
            .with_title(format!(" {} Weather History", location.name)),
            criteria: CriteriaDialog::new(),
            location_name: location.name.clone(),
            location_alias: location.alias.clone(),
            weather_data,
        }
    }
    /// Get the size of the report dialog.
    ///
    pub fn size(&self) -> Size {
        self.dialog.win().size()
    }
    /// Force the dialog to recreate the history report view.
    ///
    fn refresh(&mut self) {
        match self.criteria.try_as_date_range() {
            Err(error_message) => {
                self.dialog.set_message(MessageStyle::Error, error_message);
            }
            Ok(date_range) => {
                self.criteria.set_active(false);
                // self.view.take();
                let criteria = DataCriteria::default().filters(vec![self.location_alias.clone()]);
                match self.weather_data.get_daily_history(criteria, date_range) {
                    Err(error_message) => {
                        let message = format!("Failed to get daily history ({}).", error_message);
                        log::error!("{}", message);
                        self.dialog.set_message(MessageStyle::Error, message);
                    }
                    Ok(daily_histories) => match self.criteria.try_as_controller() {
                        Err(error_message) => {
                            self.dialog.set_message(MessageStyle::Error, error_message);
                        }
                        Ok(controller) => {
                            let report = Report::new(controller).with_date_format("%m/%d/%Y");
                            self.dialog.win_mut().set_view(
                                ReportView::new(report.generate(daily_histories), None)
                                    .with_show_selected(true)
                                    .with_column_labels(true)
                                    .with_horizontal_scroll(true),
                            );
                        }
                    },
                }
            }
        }
    }
    /// Dispatch a key pressed event to the report view dialog. [ControlFlow::Continue] will be
    /// returned if the event is not consumed.
    ///
    /// # Arguments
    ///
    /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
    ///
    pub fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
        log_key_pressed!("ReportDialog");
        match self.criteria.is_active() {
            true => {
                match self.criteria.key_pressed(key_event) {
                    ControlFlow::Break(DialogResult::Continue) => (),
                    ControlFlow::Break(DialogResult::Cancel) => match self.dialog.win().view.is_some() {
                        true => self.criteria.set_active(false),
                        false => break_event!(DialogResult::Exit)?,
                    },
                    ControlFlow::Break(DialogResult::Exit) => {
                        self.criteria.set_active(false);
                        self.refresh();
                    }
                    ControlFlow::Continue(_) => beep(),
                    unknown => {
                        debug_assert!(false, "missed criteria result {:?}", unknown);
                        log::error!("Yikes... missed criteria result {:?}", unknown)
                    }
                }
                break_event!(DialogResult::Continue)?;
            }
            false => {
                if let ControlFlow::Break(dialog_result) = self.dialog.key_pressed(key_event) {
                    match dialog_result {
                        DialogResult::Cancel => break_event!(DialogResult::Exit)?,
                        DialogResult::Selected(id) => match id.as_str() {
                            EXIT_ID => break_event!(DialogResult::Exit)?,
                            CRITERIA_ID => {
                                // self.dialog.win_mut().set_active(true);
                                self.criteria.set_active(true);
                                break_event!(DialogResult::Continue)?;
                            }
                            _ => unreachable!(),
                        },
                        result => {
                            if DialogResult::Continue != result {
                                debug_assert!(false, "missed dialog result {:?}", result);
                                log::error!("key_pressed missed {:#?}", result);
                            }
                            break_event!(result)?
                        }
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
    /// Draw the report view dialog on the terminal screen, optionally returning the current
    /// cursor position.
    ///
    /// # Arguments
    ///
    /// - `area` is where on the terminal screen the window will be drawn.
    /// - `buffer` is the current view of the terminal screen.
    ///
    pub fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
        log_render!("ReportDialog");
        debug_assert!(
            self.dialog.win().view.is_some() || self.criteria.is_active(),
            "ReportDialog bad state, neither window or criteria active\n{:#?}",
            self
        );
        let mut coord = None;
        // if you don't have a view, don't render the dialog
        if self.dialog.win().view.is_some() {
            if let Some(dialog_coord) = self.dialog.render(area, buffer) {
                coord.replace(dialog_coord);
            }
        }
        if self.criteria.is_active() {
            if let Some(criteria_coord) = self.criteria.render(area, buffer) {
                coord.replace(criteria_coord);
            }
        }
        coord
    }
}

/// The location history report window.
///
#[derive(Debug, Default)]
struct ReportWindow {
    /// Indicates the window is active or not.
    active: bool,
    /// The location history report view.
    view: Option<ReportView>,
}
impl ReportWindow {
    /// Change the report view that will be drawn.
    ///
    /// # Arguments
    ///
    /// - `view` is the new report view.
    ///
    fn set_view(&mut self, view: ReportView) {
        self.view.replace(view);
    }
}
impl DialogWindow for ReportWindow {
    /// Query if the report view is active or not.
    ///
    fn is_active(&self) -> bool {
        self.active
    }
    /// Control if the report view is active or not.
    ///
    /// # Arguments
    ///
    /// - `yes_no` determines if the dialog is active or not.
    ///
    fn set_active(&mut self, yes_no: bool) {
        self.active = yes_no;
    }
    /// Get the size of the report view.
    ///
    fn size(&self) -> Size {
        match self.view.as_ref() {
            None => Size::default(),
            Some(view) => view.size(),
        }
    }
    /// Dispatch a key pressed event to the report view window. [ControlFlow::Continue] will be
    /// returned if the event is not consumed.
    ///
    /// # Arguments
    ///
    /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
    ///
    fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
        log_key_pressed!("ReportWindow");
        if let Some(view) = self.view.as_mut() {
            if let ControlFlow::Break(control_result) = view.key_pressed(&key_event) {
                if ControlResult::NotAllowed == control_result {
                    beep();
                }
                break_event!(DialogResult::Continue)?;
            }
        }
        ControlFlow::Continue(())
    }
    /// Draw the report view window on the terminal screen, optionally returning the current
    /// cursor position.
    ///
    /// # Arguments
    ///
    /// - `area` is where on the terminal screen the window will be drawn.
    /// - `buffer` is the current view of the terminal screen.
    ///
    fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
        log_render!("ReportWindow");
        let view = self.view.as_ref()?;
        let coord = view.render(area, buffer, view.catalog_type.get_styles(ControlState::Active))?;
        Some(coord)
    }
}

mod criteria {
    //! The select report categories dialog.
    use std::mem::discriminant;

    use super::*;

    /// The start date identifier.
    ///
    const START_ID: &'static str = "START";

    /// The end date identifier.
    ///
    const END_ID: &'static str = "END";

    /// The temperature report content identifier.
    ///
    const TEMPERATURE_ID: &'static str = "TEMP";

    /// The precipitation report content identifier.
    ///
    const PRECIPITATION_ID: &'static str = "PRECIP";

    /// The conditions report content identifier.
    ///
    const CONDITIONS_ID: &'static str = "COND";

    /// The summary report content identifier.
    ///
    const SUMMARY_ID: &'static str = "SUM";

    #[derive(Debug)]
    pub struct CriteriaDialog(ButtonDialog<CriteriaWindow>);
    impl CriteriaDialog {
        /// Create a new instance of the report criteria dialog.
        ///
        pub fn new() -> Self {
            Self(
                ButtonDialog::new(
                    ButtonBar::new(vec![ok_button().with_active()]).with_auto_select(true),
                    CriteriaWindow::new(),
                )
                .with_title(" History Report Criteria "),
            )
        }
        /// Dispatch a key pressed event to the report criteria dialog.
        /// [ControlFlow::Continue] will be returned if the event is not consumed.
        ///
        /// # Arguments
        ///
        /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
        ///
        pub fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
            log_key_pressed!("CriteriaDialog");
            match self.0.key_pressed(key_event) {
                ControlFlow::Break(DialogResult::Selected(_)) => match self.0.win_mut().try_as_date_range() {
                    Err(error_message) => {
                        self.0.set_message(MessageStyle::Error, error_message);
                        break_event!(DialogResult::Continue)
                    }
                    Ok(_) => match self.0.win().try_as_report_selector() {
                        Ok(_) => {
                            self.set_active(false);
                            break_event!(DialogResult::Exit)
                        },
                        Err(error_message) => {
                            self.0.set_message(MessageStyle::Error, error_message);
                            break_event!(DialogResult::Continue)
                        }
                    },
                },
                ControlFlow::Break(DialogResult::Cancel) => {
                    self.set_active(false);
                    break_event!(DialogResult::Cancel)
                }
                ControlFlow::Continue(()) => {
                    beep();
                    break_event!(DialogResult::Continue)
                }
                result => result,
            }
        }
        /// Draw the report criteria dialog on the terminal screen, optionally returning the current
        /// cursor position.
        ///
        /// # Arguments
        ///
        /// - `area` is where on the terminal screen the window will be drawn.
        /// - `buffer` is the current view of the terminal screen.
        ///
        pub fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
            log_render!("CriteriaDialog");
            self.0.render(area, buffer)
        }
        /// Query if the report criteria dialog is active or not.
        ///
        pub fn is_active(&self) -> bool {
            self.0.win().is_active()
        }
        /// Control if the report criteria dialog is active or not.
        ///
        /// # Arguments
        ///
        /// - `yes_no` determines if the dialog is active or not.
        ///
        pub fn set_active(&mut self, yes_no: bool) {
            self.0.win_mut().set_active(yes_no)
        }
        /// Try to get the report [date range](DateRange) from the dialog.
        ///
        pub fn try_as_date_range(&mut self) -> std::result::Result<DateRange, String> {
            self.0.win_mut().try_as_date_range()
        }
        /// Try to get the report [content selection](ReportSelector) from the dialog.
        ///
        pub fn try_as_controller(&mut self) -> std::result::Result<ReportSelector, String> {
            self.0.win_mut().try_as_report_selector()
        }
    }

    /// The report criteria window.
    #[derive(Debug)]
    struct CriteriaWindow {
        /// Indicates the window is active or not.
        active: bool,
        /// The report start and end dates.
        dates: EditFieldGroup,
        /// The report content selection.
        criteria: CheckBoxGroup,
        /// The size of the window.
        size: Size,
    }
    impl CriteriaWindow {
        /// Create a new instance of the criteria window.
        ///
        fn new() -> Self {
            let date_str = "MM/DD/YYYY";
            let dates = EditFieldGroup::new(vec![
                EditField::new(
                    Label::align_right("Starting: ").with_id(START_ID).with_selector('S').with_active(),
                    DateEditor::default(),
                ),
                EditField::new(
                    Label::align_right("Ending: ").with_id(END_ID).with_selector('E'),
                    DateEditor::default(),
                ),
            ])
            .with_labels_aligned()
            .with_centered_fields()
            .with_title(format!("Report Dates ({})", date_str))
            .with_title_alignment(Alignment::Center)
            .with_active();
            let criteria = CheckBoxGroup::new(vec![
                Checkbox::new(TEMPERATURE_ID, "Temperatures", 'T'),
                Checkbox::new(PRECIPITATION_ID, "Precipitation", 'P'),
                Checkbox::new(CONDITIONS_ID, "Conditions", 'n'),
                Checkbox::new(SUMMARY_ID, "Summary", 'u'),
            ])
            .with_labels_aligned()
            .with_centered_fields()
            .with_wrap()
            .with_title("Report Categories")
            .with_title_alignment(Alignment::Center);
            let dates_size = dates.size();
            let criteria_size = criteria.size();
            let size = Size {
                width: cmp::max(dates_size.width, criteria_size.width),
                height: dates_size.height + criteria_size.height + 1,
            };
            Self { active: true, dates, criteria, size }
        }
        /// Try to get the report [date range](DateRange) from the window.
        ///
        fn try_as_date_range(&mut self) -> std::result::Result<DateRange, String> {
            match validate_date("From", self.dates.get_mut(START_ID).unwrap().text()) {
                Err(parse_error) => {
                    self.dates.set_active(START_ID);
                    Err(parse_error)
                }
                Ok(start) => match validate_date("Through", self.dates.get(END_ID).unwrap().text()) {
                    Err(parse_error) => {
                        self.dates.set_active(END_ID);
                        Err(parse_error)
                    }
                    Ok(end) => match start <= end {
                        false => Err(format!("Start date {} cannot be before end date {}", start, end)),
                        true => Ok(DateRange::new(start, end))
                    },
                },
            }
        }
        /// Try to get the report [content selection](ReportSelector) from the window.
        ///
        fn try_as_report_selector(&self) -> std::result::Result<ReportSelector, String> {
            let temperatures = self.criteria.get(TEMPERATURE_ID).unwrap().is_checked();
            let precipitation = self.criteria.get(PRECIPITATION_ID).unwrap().is_checked();
            let conditions = self.criteria.get(CONDITIONS_ID).unwrap().is_checked();
            let summary = self.criteria.get(SUMMARY_ID).unwrap().is_checked();
            match temperatures || precipitation || conditions || summary {
                true => Ok(ReportSelector { temperatures, precipitation, conditions, summary }),
                false => Err("A report category must be selected.".to_string()),
            }
        }
    }
    impl DialogWindow for CriteriaWindow {
        /// Query if the report criteria window is active or not.
        ///
        fn is_active(&self) -> bool {
            self.active
        }
        /// Control if the report criteria window is active or not.
        ///
        /// # Arguments
        ///
        /// - `yes_no` determines if the dialog is active or not.
        ///
        fn set_active(&mut self, yes_no: bool) {
            self.active = yes_no;
        }
        /// Get the size of the report criteria window.
        ///
        fn size(&self) -> Size {
            self.size
        }
        /// Dispatch a key pressed event to the report criteria window. [ControlFlow::Continue] will be
        /// returned if the event is not consumed.
        ///
        /// # Arguments
        ///
        /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
        ///
        fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
            log_key_pressed!("CriteriaWindow");
            macro_rules! toggle_active_group {
                () => {
                    self.dates.active = !self.dates.active;
                    self.criteria.active = !self.dates.active;
                };
            }
            // check the event to see if it is a field selector
            let is_selector = match key_event.modifiers == KeyModifiers::ALT {
                true => discriminant(&key_event.code) == discriminant(&KeyCode::Char(' ')),
                false => false,
            };
            let control_result = match self.dates.active {
                true => match self.dates.key_pressed(key_event) {
                    ControlFlow::Continue(_) => match is_selector {
                        false => ControlFlow::Continue(()),
                        true => {
                            // if dates didn't handle the event then try criteria
                            let criteria_result = self.criteria.key_pressed(key_event);
                            if criteria_result.is_break() {
                                toggle_active_group!();
                                self.dates.clear_active();
                            }
                            criteria_result
                        }
                    },
                    dates_result => dates_result,
                },
                false => match self.criteria.key_pressed(key_event) {
                    ControlFlow::Continue(_) => match is_selector {
                        false => ControlFlow::Continue(()),
                        true => {
                            // if criteria didn't handle the event then try dates
                            let dates_result = self.dates.key_pressed(key_event);
                            if dates_result.is_break() {
                                toggle_active_group!();
                                self.criteria.clear_active();
                            }
                            dates_result
                        }
                    },
                    criteria_result => criteria_result,
                },
            };
            if let ControlFlow::Break(control_result) = control_result {
                // log::debug!("control result {:?}", control_result);
                match control_result {
                    ControlResult::Continue => (),
                    ControlResult::NotAllowed => beep(),
                    ControlResult::Selected(id) => {
                        let id_str = id.as_str();
                        if id_str == START_ID || id_str == END_ID {
                            self.dates.set_active(id);
                        } else {
                            self.criteria.set_active(id);
                        }
                    }
                    ControlResult::NextGroup => {
                        if self.dates.active {
                            self.dates.clear_active();
                            self.criteria.set_first_active();
                        } else {
                            self.criteria.clear_active();
                            self.dates.set_first_active();
                        }
                        toggle_active_group!();
                    }
                    ControlResult::PrevGroup => {
                        if self.dates.active {
                            self.dates.clear_active();
                            self.criteria.set_last_active();
                        } else {
                            self.criteria.clear_active();
                            self.dates.set_last_active();
                        }
                        toggle_active_group!();
                    }
                    unknown => {
                        debug_assert!(false, "control result not handled {:?}", unknown);
                        log::error!("window result not handled {:?}", unknown);
                    }
                }
                break_event!(DialogResult::Continue)?;
            }
            ControlFlow::Continue(())
        }
        /// Draw the report criteria window on the terminal screen, optionally returning the current
        /// cursor position.
        ///
        /// # Arguments
        ///
        /// - `area` is where on the terminal screen the window will be drawn.
        /// - `buffer` is the current view of the terminal screen.
        ///
        fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
            if !self.active {
                None?;
            }
            log_render!("CriteriaWindow");
            // show the date group
            let dates_height = self.dates.size().height as i32;
            let dates_area = inner_rect(area, (0, 0), (0, dates_height));
            let styles = match self.dates.active {
                true => ActiveNormalStyles::new(self.dates.catalog_type),
                false => ActiveNormalStyles::with_active_style(self.dates.catalog_type, ControlState::Normal),
            };
            let mut coord = self.dates.render(dates_area, buffer, styles);
            // show the criteria group
            let criteria_area = inner_rect(area, (0, dates_height + 1), (0, 0));
            let styles = match self.criteria.active {
                true => ActiveNormalStyles::new(self.criteria.catalog_type),
                false => ActiveNormalStyles::with_active_style(self.dates.catalog_type, ControlState::Normal),
            };
            if let Some(criteria_coord) = self.criteria.render(criteria_area, buffer, styles) {
                coord.replace(criteria_coord);
            }
            coord
        }
    }
}
