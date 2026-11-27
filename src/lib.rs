//! The system's own controls for Native Shell apps (an nshm): its
//! segmented control ([`SegmentedControl`], iOS's), switch ([`Switch`]) and
//! date and time picker ([`DateTimePicker`], iOS's and Android's), each in
//! the system's exact look, motion, haptics and accessibility.
//!
//! TSX imports them from `nsh-system-controls`; Rust makes them with
//! [`segmented_control()`], [`switch()`] and [`date_time_picker()`]. Each
//! is a native component this crate registers on iOS
//! (`#[native_components(ios)]`, UIKit's controls) and Android
//! (`#[native_components(android)]`, the framework's views in the crate's
//! Java library). Where a platform has no such control, the element takes
//! no room and shows nothing, so one layout holds both: the app shows
//! `nsh/base`'s controls there, which also suit an app that styles its
//! controls itself.

mod date_time_picker;
mod segmented_control;
mod switch;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use nsh_core::{
    JsonValue,
    element::{NativeView, global},
    i18n::Translations,
    platform::Os,
    prelude::*,
};

pub use crate::{
    date_time_picker::{
        DateTimePicker, DateTimePickerElement, DateTimePickerEvent, DateTimePickerMode,
        DateTimePickerProps, DateTimePickerValue, date_time_picker,
    },
    segmented_control::{
        SegmentedControl, SegmentedControlElement, SegmentedControlEvent, SegmentedControlItem,
        SegmentedControlProps, segmented_control,
    },
    switch::{Switch, SwitchElement, SwitchEvent, SwitchProps, switch},
};

/// The native component of [`SegmentedControl`], which this crate
/// registers on iOS.
pub const SEGMENTED_CONTROL_COMPONENT: &str = "nsh-system-controls/segmented-control";
/// The native component of [`Switch`], which this crate registers on iOS
/// and Android.
pub const SWITCH_COMPONENT: &str = "nsh-system-controls/switch";
/// The native component of [`DateTimePicker`], which this crate registers
/// on iOS and Android.
pub const DATE_TIME_PICKER_COMPONENT: &str = "nsh-system-controls/date-time-picker";

#[native_module("nsh-system-controls")]
fn native(registry: &mut Registry) {
    registry
        .register::<SegmentedControl>()
        .register::<Switch>()
        .register::<DateTimePicker>();
}

/// UIKit's controls, for the views of the hosts nsh builds for the app.
#[native_components(ios)]
fn components(components: &nsh_platform_ios::Components) {
    ios::register(components);
}

/// The framework's views, for the views of the hosts nsh builds for the
/// app.
#[native_components(android)]
fn components(components: &nsh::Components) {
    android::register(components);
}

/// The theme's `primary` in the appearance shown, as the controls take
/// their color (`tint`); `null` without one. Reading it subscribes to the
/// theme and the appearance.
fn primary_tint() -> JsonValue {
    tokens()
        .color("primary")
        .map_or(JsonValue::Null, |primary| color(primary).to_hex().into())
}

/// `element` with what the system's views cannot take from the system,
/// each sent again when it changes: on Android the appearance shown
/// (`dark`: the app's, else the system's), and with `locale` on iOS and
/// Android the language of the app's text (`locale`, a BCP 47 tag). UIKit
/// follows the appearance by itself; its date picker is in the user's
/// locale unless told the app's language, and keeps the user's where the
/// two are the same language.
fn system_environment(element: NativeView, locale: bool) -> NativeView {
    let element = if os() == Os::Android {
        element.prop("dark", || JsonValue::Bool(dark()))
    } else {
        element
    };
    if locale && matches!(os(), Os::Android | Os::Ios) {
        element.prop("locale", || JsonValue::from(text_locale().as_str()))
    } else {
        element
    }
}

/// The language the app's text shows in, which NSH's own words on the
/// platform follow too: the first of the user's languages the app has
/// translations for (else the user's locale), and with translations, the
/// one of theirs it falls back to. Reading it subscribes to the language
/// and the translations.
fn text_locale() -> nsh_core::platform::Locale {
    let translations = global::<Translations>().get();
    let languages = locale_settings().languages;
    let shown = translations.pick(languages.iter()).unwrap_or_else(locale);
    if translations.locales().is_empty() {
        shown
    } else {
        translations.pick([shown]).unwrap_or_default()
    }
}
