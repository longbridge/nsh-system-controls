//! The controls run headless in an engine with a Rust root: what each
//! element sends its native component, the user's choices coming back as
//! the component's events, and where the controls take no room.

use std::{
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use nsh_core::{
    ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem,
    element::{Root, declarations},
    platform::{Appearance, Locale, Os, is_reserved_component},
    prelude::*,
};
use nsh_engine::{Engine, EngineConfig, Environment, MountSink};
use nsh_system_controls::{
    DATE_TIME_PICKER_COMPONENT, DateTimePickerMode, DateTimePickerValue,
    SEGMENTED_CONTROL_COMPONENT, SWITCH_COMPONENT, SegmentedControlItem, date_time_picker,
    segmented_control, switch,
};
use serde_json::{Value, json};

struct ChannelSink(Sender<MountBatch>);

impl MountSink for ChannelSink {
    fn submit(&self, batch: MountBatch) {
        let _ = self.0.send(batch);
    }
}

/// Runs `root` on a phone of `os`.
fn start(root: Root, os: Os) -> (Engine, Receiver<MountBatch>) {
    let (tx, rx) = channel();
    let config = EngineConfig {
        environment: Environment {
            os,
            ..Environment::default()
        },
        ..EngineConfig::new(root)
    };
    let engine = Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap();
    (engine, rx)
}

/// The batches until `done` holds for one (or nothing came for 500ms).
fn until(rx: &Receiver<MountBatch>, mut done: impl FnMut(&MountBatch) -> bool) -> Vec<MountBatch> {
    let mut batches = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(500)) {
        let stop = done(&batch);
        batches.push(batch);
        if stop {
            break;
        }
    }
    batches
}

/// Whether `batch` shows the text `wanted`.
fn shows(batch: &MountBatch, wanted: &str) -> bool {
    batch
        .items
        .iter()
        .any(|item| matches!(item, MountItem::Text { text, .. } if text.text == wanted))
}

/// The ids of `component` mounted in `batches`, in order.
fn components(batches: &[MountBatch], component: &str) -> Vec<u32> {
    batches
        .iter()
        .flat_map(|batch| &batch.items)
        .filter_map(|item| match item {
            MountItem::Component { id, name } if &**name == component => Some(*id),
            _ => None,
        })
        .collect()
}

/// The value the prop `key` of element `id` last took in `batches`.
fn component_prop(batches: &[MountBatch], id: u32, key: &str) -> Option<Value> {
    batches
        .iter()
        .flat_map(|batch| &batch.items)
        .filter_map(|item| match item {
            MountItem::ComponentProps { id: c, props } if *c == id => Some(props),
            _ => None,
        })
        .flatten()
        .rfind(|(name, _)| &**name == key)
        .and_then(|(_, value)| value.clone())
}

/// The user's action on native component `id`: its event `name`.
fn native_event(engine: &Engine, id: u32, name: &str, data: Value) {
    engine.control().dispatch_event(
        id,
        EventKind::NativeEvent,
        EventPayload::Native {
            name: name.into(),
            data,
        },
    );
}

// --- SegmentedControl --------------------------------------------------------

#[component]
fn Segments() -> impl IntoElement {
    let selected = signal(0u32);
    let items = signal(vec![
        SegmentedControlItem::new("Light").icon("sun.max"),
        SegmentedControlItem::new("Dark"),
    ]);
    view()
        .child(
            segmented_control()
                .items(move || items.get())
                .selected_index(move || selected.get())
                .on_change(move |index: &u32| {
                    selected.set(*index);
                    items.set(vec![
                        SegmentedControlItem::new("Light").icon("sun.max"),
                        SegmentedControlItem::new("Dark").disabled(true),
                    ]);
                }),
        )
        .child(text(move || format!("Selected: {}", selected.get())))
}

/// The segmented control is the system's on iOS: its segments and
/// selection reach it, the user's choice comes back as `onChange` and the
/// app's new props reach the same control.
#[test]
fn the_segmented_control_is_the_systems_on_ios_and_reports_the_choice() {
    let (engine, rx) = start(Root::of::<Segments>(), Os::Ios);
    let initial = until(&rx, |batch| shows(batch, "Selected: 0"));
    let control = components(&initial, SEGMENTED_CONTROL_COMPONENT)[0];
    assert_eq!(
        component_prop(&initial, control, "items"),
        Some(json!([{ "title": "Light", "icon": "sun.max" }, { "title": "Dark" }]))
    );
    assert_eq!(
        component_prop(&initial, control, "selectedIndex"),
        Some(json!(0))
    );
    assert_eq!(
        component_prop(&initial, control, "disabled"),
        Some(json!(false))
    );
    native_event(&engine, control, "change", json!(1));
    let updates = until(&rx, |batch| shows(batch, "Selected: 1"));
    assert_eq!(
        component_prop(&updates, control, "selectedIndex"),
        Some(json!(1))
    );
    assert_eq!(
        component_prop(&updates, control, "items"),
        Some(json!([
            { "title": "Light", "icon": "sun.max" },
            { "title": "Dark", "disabled": true },
        ]))
    );
}

/// Elsewhere the segmented control takes no room and makes no component.
#[test]
fn the_segmented_control_shows_nothing_off_ios() {
    for os in [Os::Android, Os::MacOs, Os::Web] {
        let (_engine, rx) = start(Root::of::<Segments>(), os);
        let batches = until(&rx, |batch| shows(batch, "Selected: 0"));
        assert!(
            components(&batches, SEGMENTED_CONTROL_COMPONENT).is_empty(),
            "{os:?}"
        );
    }
}

// --- Switch ------------------------------------------------------------------

#[component]
fn Switches() -> impl IntoElement {
    let on = signal(false);
    let locked = signal(false);
    view()
        .child(
            switch()
                .checked(move || on.get())
                .accessibility_label(Some("Notifications".into()))
                .on_change(move |checked: &bool| on.set(*checked)),
        )
        .child(
            switch()
                .checked(move || locked.get())
                .disabled(true)
                .accessibility_label(Some("Locked".into()))
                .on_change(|_: &bool| {}),
        )
        .child(text(move || format!("On: {}", on.get())))
}

/// The switch is the system's on iOS and Android: its value, `disabled`
/// and label reach it, the user's flip comes back as `onChange`, the app's
/// new value reaches the same switch, and a flip the app has heard bumps
/// `revision`, so the switch shows a value the app kept again.
#[test]
fn the_switch_is_the_systems_on_mobile_and_follows_the_app() {
    for os in [Os::Ios, Os::Android] {
        let (engine, rx) = start(Root::of::<Switches>(), os);
        let initial = until(&rx, |batch| shows(batch, "On: false"));
        let ids = components(&initial, SWITCH_COMPONENT);
        assert_eq!(ids.len(), 2, "two switches on {os:?}");
        // Each switch by its label: the mount order is not the tree's.
        let by_label = |label: &str| {
            *ids.iter()
                .find(|&&id| {
                    component_prop(&initial, id, "accessibilityLabel") == Some(json!(label))
                })
                .unwrap_or_else(|| panic!("the {label} switch on {os:?}"))
        };
        let (on, locked) = (by_label("Notifications"), by_label("Locked"));
        assert_eq!(component_prop(&initial, on, "checked"), Some(json!(false)));
        assert_eq!(component_prop(&initial, on, "disabled"), Some(json!(false)));
        assert_eq!(
            component_prop(&initial, locked, "disabled"),
            Some(json!(true))
        );
        // No theme here: the system's color.
        assert_eq!(component_prop(&initial, on, "tint"), Some(json!(null)));
        // The app follows the flip.
        native_event(&engine, on, "change", json!(true));
        let updates = until(&rx, |batch| shows(batch, "On: true"));
        assert_eq!(
            component_prop(&updates, on, "checked"),
            Some(json!(true)),
            "{os:?}"
        );
        // The app keeps its value: the switch is told it again.
        native_event(&engine, locked, "change", json!(true));
        let kept = until(&rx, |batch| {
            component_prop(std::slice::from_ref(batch), locked, "revision").is_some()
        });
        assert_eq!(
            component_prop(&kept, locked, "revision"),
            Some(json!(1)),
            "{os:?}"
        );
        assert_eq!(
            component_prop(&kept, locked, "checked"),
            None,
            "the kept value stays as it was on {os:?}"
        );
    }
}

/// The desktops and the web have no system switch: it takes no room and
/// makes no component.
#[test]
fn the_switch_shows_nothing_off_mobile() {
    for os in [Os::MacOs, Os::Windows, Os::Linux, Os::Web] {
        let (_engine, rx) = start(Root::of::<Switches>(), os);
        let batches = until(&rx, |batch| shows(batch, "On: false"));
        assert!(components(&batches, SWITCH_COMPONENT).is_empty(), "{os:?}");
    }
}

// --- DateTimePicker ----------------------------------------------------------

#[component]
fn Pickers() -> impl IntoElement {
    let value = signal(DateTimePickerValue::parse("2026-11-17T09:30").unwrap());
    view()
        .child(
            date_time_picker()
                .mode(DateTimePickerMode::DateTime)
                .value(move || Some(value.get()))
                .min(DateTimePickerValue::parse("2026-11-01T00:00"))
                // Another mode's value: not passed on.
                .max(DateTimePickerValue::parse("18:00"))
                .minute_interval(7)
                .accessibility_label(Some("Reminder".into()))
                .on_change(move |picked: &DateTimePickerValue| value.set(*picked)),
        )
        .child(
            date_time_picker()
                .mode(DateTimePickerMode::Time)
                .value(DateTimePickerValue::parse("09:30")),
        )
        .child(text(move || format!("Picked: {}", value.get().iso())))
}

/// The date and time picker is the system's on iOS and Android: its value
/// and bounds reach it as fields, a minute step that does not divide an
/// hour and a bound of another mode are not passed on, its
/// `accessibilityLabel` reaches it (none without one), and the user's pick
/// comes back as `onChange` in the mode's form.
#[test]
fn the_date_time_picker_is_the_systems_on_mobile_and_reports_the_pick() {
    for os in [Os::Ios, Os::Android] {
        let (engine, rx) = start(Root::of::<Pickers>(), os);
        let initial = until(&rx, |batch| shows(batch, "Picked: 2026-11-17T09:30"));
        // Each picker by its mode: the mount order is not the tree's.
        let by_mode = |mode: &str| {
            components(&initial, DATE_TIME_PICKER_COMPONENT)
                .into_iter()
                .find(|&id| component_prop(&initial, id, "mode") == Some(json!(mode)))
                .unwrap_or_else(|| panic!("the {mode} picker on {os:?}"))
        };
        let (picker, unnamed) = (by_mode("dateTime"), by_mode("time"));
        assert_eq!(
            component_prop(&initial, picker, "value"),
            Some(json!({ "year": 2026, "month": 11, "day": 17, "hour": 9, "minute": 30 }))
        );
        assert_eq!(
            component_prop(&initial, picker, "min"),
            Some(json!({ "year": 2026, "month": 11, "day": 1, "hour": 0, "minute": 0 }))
        );
        assert_eq!(component_prop(&initial, picker, "max"), Some(json!(null)));
        assert_eq!(
            component_prop(&initial, picker, "minuteInterval"),
            Some(json!(1))
        );
        assert_eq!(
            component_prop(&initial, picker, "accessibilityLabel"),
            Some(json!("Reminder")),
            "{os:?}"
        );
        assert_eq!(
            component_prop(&initial, unnamed, "accessibilityLabel"),
            Some(json!(null)),
            "{os:?}"
        );
        assert_eq!(
            component_prop(&initial, unnamed, "value"),
            Some(json!({ "hour": 9, "minute": 30 }))
        );
        native_event(
            &engine,
            picker,
            "change",
            json!({ "year": 2026, "month": 12, "day": 1, "hour": 18, "minute": 45 }),
        );
        let updates = until(&rx, |batch| shows(batch, "Picked: 2026-12-01T18:45"));
        assert_eq!(
            component_prop(&updates, picker, "value"),
            Some(json!({ "year": 2026, "month": 12, "day": 1, "hour": 18, "minute": 45 })),
            "{os:?}"
        );
    }
}

/// The desktops and the web have no system date picker: it takes no room
/// and makes no component.
#[test]
fn the_date_time_picker_shows_nothing_off_mobile() {
    for os in [Os::MacOs, Os::Windows, Os::Linux, Os::Web] {
        let (_engine, rx) = start(Root::of::<Pickers>(), os);
        let batches = until(&rx, |batch| shows(batch, "Picked: 2026-11-17T09:30"));
        assert!(
            components(&batches, DATE_TIME_PICKER_COMPONENT).is_empty(),
            "{os:?}"
        );
    }
}

// --- What Android's views draw by ---------------------------------------------

#[component]
fn InAnotherLanguage() -> impl IntoElement {
    let shown = signal(false);
    spawn(async move {
        nsh_core::i18n::set_locale(Locale::parse("de-DE"));
        shown.set(true);
    });
    view()
        .child(switch())
        .child(date_time_picker())
        .child(text(move || format!("German: {}", shown.get())))
}

/// Android's views draw Material's colors and format the value themselves,
/// so they are told the appearance shown, the app's over the system's
/// (`dark`), and the picker the language of the app's text (`locale`),
/// again as it changes. UIKit's controls take both from the system.
#[test]
fn android_controls_follow_the_apps_appearance_and_language() {
    let root = || Root::of::<InAnotherLanguage>().appearance(Appearance::Dark);
    let (_engine, rx) = start(root(), Os::Android);
    let batches = until(&rx, |batch| shows(batch, "German: true"));
    let switch = components(&batches, SWITCH_COMPONENT)[0];
    let picker = components(&batches, DATE_TIME_PICKER_COMPONENT)[0];
    assert_eq!(component_prop(&batches, switch, "dark"), Some(json!(true)));
    assert_eq!(component_prop(&batches, picker, "dark"), Some(json!(true)));
    assert_eq!(component_prop(&batches, switch, "locale"), None);
    assert_eq!(
        component_prop(&batches, picker, "locale"),
        Some(json!("de-DE"))
    );

    let (_engine, rx) = start(root(), Os::Ios);
    let batches = until(&rx, |batch| shows(batch, "German: true"));
    for id in [
        components(&batches, SWITCH_COMPONENT)[0],
        components(&batches, DATE_TIME_PICKER_COMPONENT)[0],
    ] {
        assert_eq!(component_prop(&batches, id, "dark"), None);
        assert_eq!(component_prop(&batches, id, "locale"), None);
    }
}

// --- The TSX face ------------------------------------------------------------

/// TSX imports the controls from `nsh-system-controls` with the props,
/// events and types they had in `nsh/mobile`, and their native components
/// are the crate's own: names under `nsh/` are NSH's.
#[test]
fn tsx_gets_the_controls_with_their_props_and_types() {
    let registry = nsh_system_controls::__nsh_native_module();
    let declared = declarations(&registry, &[]);
    for wanted in [
        "export declare const SegmentedControl: ((props: SegmentedControlProps) => JSX.Element);",
        "export interface SegmentedControlItem {",
        "  items?: (SegmentedControlItem)[];",
        "  selectedIndex?: number;",
        "  onChange?: (event: { index: number }) => void;",
        "export declare const Switch: ((props: SwitchProps) => JSX.Element);",
        "  checked?: boolean;",
        "  accessibilityLabel?: string | null;",
        "  onChange?: (event: { checked: boolean }) => void;",
        "export declare const DateTimePicker: ((props: DateTimePickerProps) => JSX.Element);",
        "export type DateTimePickerMode = \"date\" | \"time\" | \"dateTime\";",
        "  value?: string | null;",
        "  minuteInterval?: number;",
        "  onChange?: (event: { value: string }) => void;",
    ] {
        assert!(declared.contains(wanted), "no {wanted} in\n{declared}");
    }
    for name in [
        SEGMENTED_CONTROL_COMPONENT,
        SWITCH_COMPONENT,
        DATE_TIME_PICKER_COMPONENT,
    ] {
        assert!(!is_reserved_component(name), "{name}");
    }
}
