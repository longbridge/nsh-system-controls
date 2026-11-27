//! The controls on iOS: UIKit's, made for each element by the factories
//! registered on the views of the hosts nsh builds for the app.

mod date_time_picker;
mod segmented_control;
mod switch;

use nsh_platform_ios::Components;

use crate::{DATE_TIME_PICKER_COMPONENT, SEGMENTED_CONTROL_COMPONENT, SWITCH_COMPONENT};

/// Registers the controls on `components`.
pub(crate) fn register(components: &Components) {
    components.register(SEGMENTED_CONTROL_COMPONENT, segmented_control::factory());
    components.register(SWITCH_COMPONENT, switch::factory());
    components.register(DATE_TIME_PICKER_COMPONENT, date_time_picker::factory());
}
