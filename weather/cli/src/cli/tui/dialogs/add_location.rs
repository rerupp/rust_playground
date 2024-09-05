//! The dialog that adds a location.

use super::*;

/// A dialog that add a location to weather data.
pub struct AddLocation {
    /// The dialog that allows information about the location to be added or changed.
    dialog: ButtonDialog<LocationEditor>,
    /// The weather history API that will be used.
    weather_data: Rc<WeatherData>,
}
impl Debug for AddLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddLocation").field("dialog", &self.dialog).finish()
    }
}
impl AddLocation {
    /// Create a new instance of the dialog.
    ///
    /// # Arguments
    ///
    /// - `weather_data` is the weather history API that will be used.
    ///
    pub fn new(weather_data: Rc<WeatherData>) -> Self {
        let buttons = ButtonBar::new(vec![ok_button().with_active(), cancel_button()]).with_auto_select(true);
        let window = LocationEditor::new();
        let dialog = ButtonDialog::new(buttons, window).with_title(" Add Location ");
        Self { dialog, weather_data }
    }
    /// Initialize the dialog with an existing location.
    ///
    /// Arguments
    ///
    /// - `location` is the location information that will be used to initialize the dialog.
    ///
    pub fn with_location(mut self, location: &Location) -> Self {
        self.dialog.win_mut().initialize(location);
        self
    }
    /// Dispatch a key pressed event to the dialog. [ControlFlow::Continue] will be returned if the
    /// event is not consumed.
    ///
    /// # Arguments
    ///
    /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
    ///
    pub fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
        log_key_pressed!("AddLocation");
        match self.dialog.key_pressed(key_event) {
            ControlFlow::Break(DialogResult::Selected(id)) => match id.as_str() {
                CANCEL_BUTTON_ID => ControlFlow::Break(DialogResult::Cancel),
                OK_BUTTON_ID => self.add_location(),
                _ => unreachable!(),
            },
            result => {
                // log::debug!("dialog result {:?}", result);
                result
            }
        }
    }
    /// Draw the dialog on the terminal screen and return the active cursor position.
    ///
    /// # Arguments
    ///
    /// - `area` is where on the terminal screen the menu item will be drawn.
    /// - `buffer` is the current view of the terminal screen.
    ///
    pub fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
        log_render!("AddLocation");
        self.dialog.render(area, buffer)
    }
    /// Add a location using the information contained within the dialog.
    ///
    fn add_location(&mut self) -> ControlFlow<DialogResult> {
        let location = self.dialog.win().as_location();
        match validate(&location) {
            Ok(_) => match self.weather_data.add_location(location) {
                Ok(_) => ControlFlow::Break(DialogResult::Exit)?,
                Err(err) => {
                    self.dialog.win_mut().fields.set_active(NAME_ID);
                    self.dialog.set_message(MessageStyle::Error, err);
                }
            },
            Err((err, id)) => {
                self.dialog.win_mut().fields.set_active(id);
                self.dialog.set_message(MessageStyle::Error, err);
            }
        }
        break_event!(DialogResult::Continue)
    }
}

/// The field identifier for the location name.
///
const NAME_ID: &'static str = "LOCATION";

/// The field identifier for the location alias.
///
const ALIAS_ID: &'static str = "ALIAS";

/// The field identifier for the location longitude.
///
const LONGITUDE_ID: &'static str = "LONGITUDE";

/// The field identifier for the location latitude.
///
const LATITUDE_ID: &'static str = "LATITUDE";

/// The field identifier for the location timezone.
///
const TZ_ID: &'static str = "TZ";

/// The location editor manages editing location information and is called from the [AddLocation].
#[derive(Debug)]
struct LocationEditor {
    /// Indicates if the dialog is active or not.
    active: bool,
    /// The location information edit fields.
    fields: EditFieldGroup,
}
impl LocationEditor {
    /// Create a new instance of the location information editor.
    fn new() -> Self {
        let ll_width = "-###.########".len() as u16;
        Self {
            active: false,
            fields: EditFieldGroup::new(vec![
                EditField::new(
                    Label::align_right("Name: ").with_selector('N').with_id(NAME_ID).with_active(),
                    TextEditor::default().with_width(40).with_valid_chars(alphanumeric().chain("_-., ".chars())),
                ),
                EditField::new(
                    Label::align_right("Alias: ").with_selector('A').with_id(ALIAS_ID),
                    TextEditor::default()
                        .with_width(20)
                        .with_valid_chars(alphanumeric().chain("_".chars()))
                        .with_lowercase_only(),
                ),
                EditField::new(
                    Label::align_right("Longitude: ").with_selector('g').with_id(LONGITUDE_ID),
                    TextEditor::default().with_width(ll_width).with_valid_chars(digits().chain("-.".chars())),
                ),
                EditField::new(
                    Label::align_right("Latitude: ").with_selector('t').with_id(LATITUDE_ID),
                    TextEditor::default().with_width(ll_width).with_valid_chars(digits().chain("-.".chars())),
                ),
                EditField::new(
                    Label::align_right("Timezone: ").with_selector('z').with_id(TZ_ID),
                    TextEditor::default().with_width(30).with_valid_chars(alpha().chain("/_".chars())),
                ),
            ])
            .with_labels_aligned()
            .with_wrap(),
        }
    }
    /// Initialize the location edit fields using the location information.
    ///
    /// # Arguments
    ///
    /// - `location` is the location information that will be used.
    ///
    fn initialize(&mut self, location: &Location) {
        self.fields.get_mut(NAME_ID).unwrap().set_text(&location.name);
        self.fields.get_mut(ALIAS_ID).unwrap().set_text(&location.alias);
        self.fields.get_mut(LONGITUDE_ID).unwrap().set_text(&location.longitude);
        self.fields.get_mut(LATITUDE_ID).unwrap().set_text(&location.latitude);
        self.fields.get_mut(TZ_ID).unwrap().set_text(&location.tz);
        self.fields.set_active(ALIAS_ID);
    }
    /// Converts the location field information into a [Location] instance.
    ///
    fn as_location(&self) -> Location {
        Location {
            name: self.fields.get(NAME_ID).map_or("", |field| field.text()).to_string(),
            alias: self.fields.get(ALIAS_ID).map_or("", |field| field.text()).to_string(),
            longitude: self.fields.get(LONGITUDE_ID).map_or("", |field| field.text()).to_string(),
            latitude: self.fields.get(LATITUDE_ID).map_or("", |field| field.text()).to_string(),
            tz: self.fields.get(TZ_ID).map_or("", |field| field.text()).to_string(),
        }
    }
}

impl DialogWindow for LocationEditor {
    /// Query if the dialog window is active or not.
    ///
    fn is_active(&self) -> bool {
        self.active
    }
    /// Control if the window is active or not.
    ///
    /// # Arguments
    ///
    /// - `yes_no` determines if the window is active or not.
    ///
    fn set_active(&mut self, yes_no: bool) {
        self.active = yes_no;
    }
    /// Get the size of the window.
    ///
    fn size(&self) -> Size {
        self.fields.size()
    }
    /// Dispatch a key pressed event to the current edit field. [ControlFlow::Continue] will be returned if the
    /// event is not consumed.
    ///
    /// # Arguments
    ///
    /// - `key_event` is guaranteed to be a [key pressed](crossterm::event::KeyEventKind::Press) event.
    ///
    fn key_pressed(&mut self, key_event: KeyEvent) -> ControlFlow<DialogResult> {
        log_key_pressed!("LocationEditor");
        match self.fields.key_pressed(key_event) {
            ControlFlow::Break(ControlResult::Continue) => break_event!(DialogResult::Continue),
            ControlFlow::Break(ControlResult::NotAllowed) => {
                beep();
                break_event!(DialogResult::Continue)
            }
            ControlFlow::Break(control_result) => {
                debug_assert!(false, "fields unexpected result {:?}\n{:#?}", control_result, self);
                break_event!(DialogResult::Continue)
            }
            _ => ControlFlow::Continue(()),
        }
    }
    /// Draw the edit fields on the terminal screen and return the current cursor position.
    ///
    /// # Arguments
    ///
    /// - `area` is where on the terminal screen the window will be drawn.
    /// - `buffer` is the current view of the terminal screen.
    ///
    fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<Position> {
        log_render!("LocationEditor");
        self.fields.render(area, buffer, ActiveNormalStyles::new(self.fields.catalog_type))
    }
}

/// The result of a field validation.
type ValidateResult = std::result::Result<(), (String, String)>;
macro_rules! validate_error {
    ($msg:expr, $id:expr) => {
        Err(($msg.to_string(), $id.to_string()))
    };
}

/// Validate the location contents.
///
/// # Arguments
///
/// - `location` is the location information that will be validated.
///
fn validate(location: &Location) -> ValidateResult {
    match location.name.is_empty() {
        true => validate_error!("The location name cannot be empty.", NAME_ID),
        false => match location.alias.is_empty() {
            true => validate_error!("The alias name cannot be empty.", ALIAS_ID),
            false => {
                validate_longitude(&location.longitude)?;
                validate_latitude(&location.latitude)?;
                validate_tz(&location.tz)
            }
        },
    }
}

/// Validate a longitude value.
///
/// # Arguments
///
/// - `longitude` is the value that will be validated.
///
fn validate_longitude(longitude: &str) -> ValidateResult {
    match longitude.is_empty() {
        true => validate_error!("Longitude cannot be empty.", LONGITUDE_ID),
        false => match longitude.parse::<f64>() {
            Ok(longitude) => match longitude >= -180.0 && longitude <= 180.0 {
                true => Ok(()),
                false => validate_error!("Longitude must be between -180 and 180 degrees", LONGITUDE_ID),
            },
            Err(_) => validate_error!("Longitude needs to be expressed in degrees.", LONGITUDE_ID),
        },
    }
}

/// Validate a latitude value.
///
/// # Arguments
///
/// - `latitude` is the value that will be validated.
///
fn validate_latitude(latitude: &str) -> ValidateResult {
    match latitude.is_empty() {
        true => validate_error!("Latitude cannot be empty.", LATITUDE_ID),
        false => match latitude.parse::<f64>() {
            Ok(latitude) => match latitude >= -90.0 && latitude <= 90.0 {
                true => Ok(()),
                false => validate_error!("Latitude must be between -90 and 90 degrees", LATITUDE_ID),
            },
            Err(_) => validate_error!("Latitude needs to be expressed in degrees.", LATITUDE_ID),
        },
    }
}

/// Validate a timezone value.
///
/// # Arguments
///
/// - `timezone` is the value that will be validated.
///
fn validate_tz(timezone: &str) -> ValidateResult {
    match timezone.is_empty() {
        true => validate_error!("Timezone cannot be empty.", TZ_ID),
        false => match chrono_tz::TZ_VARIANTS.iter().any(|tz| tz.name() == timezone) {
            true => Ok(()),
            false => validate_error!("Timezone is not valid.", TZ_ID),
        },
    }
}
