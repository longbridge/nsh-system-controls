//! [`Switch`]: the system's switch, iOS's and Android's.

use nsh_core::{
    JsonValue,
    element::{NativeEvent, native},
    platform::Os,
    prelude::*,
};

use crate::SWITCH_COMPONENT;

/// The props of [`Switch`].
#[derive(Debug, Clone, PartialEq, Props)]
pub struct SwitchProps {
    /// On or off (default off). The app owns the value: the user's flip is
    /// reported with `onChange`, and the switch shows the value the app
    /// passes back.
    #[prop(default)]
    pub checked: Prop<bool>,
    /// It cannot be flipped (default false): the system dims it and screen
    /// readers say it is dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// What screen readers say with the switch's state: the setting it
    /// turns on (`"Notifications"`).
    pub accessibility_label: Prop<Option<SharedString>>,
}

/// The events of a [`Switch`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum SwitchEvent {
    /// The user flipped the switch: the value it asks for. The switch moves
    /// at once, as the system's does; the app sets `checked`, and a value
    /// the app keeps moves it back.
    #[args(checked)]
    Change(bool),
}

/// The system's switch, in the system's look, motion, haptics and
/// accessibility: a `UISwitch` on iOS (Liquid Glass on iOS 26 and later),
/// Material 3's switch on Android. The app says whether it is on
/// (`checked`), hears the user's flip (`onChange`), names it for screen
/// readers (`accessibilityLabel`) and places it with its `class` and
/// `style`; its size is the system's where the style leaves it open. It
/// is on in its style's `accentColor` (`accent-*`), else in the theme's
/// `primary` (the system's color without either). The
/// desktops and the web have no such control: there it takes no room and
/// shows nothing, and the app shows the setting with `nsh/base`'s
/// `Switch`.
#[component]
pub fn Switch(props: SwitchProps, cx: Scope<SwitchEvent>) -> impl IntoElement {
    if !matches!(os(), Os::Ios | Os::Android) {
        return view().hidden().into_element();
    }
    // The control moves itself under the finger. Each flip bumps
    // `revision` after the app has heard it, and the switch then shows
    // `checked` again: a value the app keeps (it turned the flip down)
    // moves it back, as an unchanged `checked` is not sent again.
    let flips = signal(0u32);
    let switch = native(SWITCH_COMPONENT)
        .prop("checked", move || JsonValue::Bool(props.checked.get()))
        .prop("revision", move || JsonValue::from(flips.get()))
        .prop("disabled", move || JsonValue::Bool(props.disabled.get()))
        .prop("accessibilityLabel", move || {
            props
                .accessibility_label
                .get()
                .map_or(JsonValue::Null, |label| label.as_ref().into())
        })
        // One color, for the appearance shown: sent again when either changes.
        .prop("tint", crate::primary_tint);
    crate::android_environment(switch, false)
        .on_native_event(move |event: &NativeEvent| {
            if &*event.name == "change"
                && let Some(checked) = event.data.as_bool()
            {
                cx.emit(SwitchEvent::Change(checked));
                flips.update(|n| *n = n.wrapping_add(1));
            }
        })
        .into_element()
}
