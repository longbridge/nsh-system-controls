# nsh-system-controls

The system's own segmented control, switch and date and time picker for [Native Shell (NSH)](https://github.com/longbridge/nsh) apps, as an [nshm](https://longbridge.github.io/nsh/docs/nshm).

Each control is the platform's: UIKit's on iOS, the framework's on Android, with the system's look, motion, haptics and screen reader support. The app owns the value, hears the user's choice with `onChange`, and places the control with its `class` and `style`; its size is the system's where the style leaves it open.

`nsh/base` already has controls that work on every platform and that the app styles itself: `Switch`, `ToggleGroup`, `RadioGroup`, `DatePicker`, `TimeField`. Use those by default. Use this package only when the app needs exactly the system's control.

| Control | iOS and Mac Catalyst | Android | macOS, Windows, Linux, web |
|---|---|---|---|
| `SegmentedControl` | `UISegmentedControl` (Liquid Glass on iOS 26 and later) | - | - |
| `Switch` | `UISwitch` (Liquid Glass on iOS 26 and later) | The framework's switch drawn as Material 3's | - |
| `DateTimePicker` | `UIDatePicker`, compact style | The value in a field that opens the system's date or time dialog | - |

Where a platform has no such control (`-`), the element takes no room and shows nothing, so one layout holds both: show `nsh/base`'s control there, chosen by `platform.os`.

## Install

```sh
nsh add nsh-system-controls                       # from crates.io
nsh add github:longbridge/nsh-system-controls     # or from GitHub
```

The hosts `nsh` builds for the app register the controls. On Android the package carries the controls' Java classes, which this crate lists in its `Cargo.toml`; the app lists nothing.

## TSX

```tsx
import { signal } from "nsh";
import { DateTimePicker, SegmentedControl, Switch } from "nsh-system-controls";

const appearance = signal(0);
const wifi = signal(true);
const at = signal("2026-11-17T09:30");

<SegmentedControl
  items={[{ title: "Light", icon: "sun.max" }, { title: "Dark", icon: "moon" }]}
  selectedIndex={appearance()}
  onChange={({ index }) => appearance.set(index)}
/>;

<Switch accessibilityLabel="Wi-Fi" checked={wifi()} onChange={({ checked }) => wifi.set(checked)} />;

<DateTimePicker
  mode="dateTime"
  value={at()}
  min="2026-11-17T00:00"
  minuteInterval={5}
  accessibilityLabel="Reminder"
  onChange={({ value }) => at.set(value)}
/>;
```

The types are exported too:

```ts
import type {
  DateTimePickerMode,
  DateTimePickerProps,
  SegmentedControlItem,
  SegmentedControlProps,
  SwitchProps,
} from "nsh-system-controls";
```

## Rust

```rust
use nsh_core::prelude::*;
use nsh_system_controls::{
    DateTimePickerMode, DateTimePickerValue, SegmentedControlItem, date_time_picker,
    segmented_control, switch,
};

#[component]
pub fn Settings() -> impl IntoElement {
    let appearance = signal(0u32);
    let wifi = signal(true);
    let at = signal(DateTimePickerValue::parse("2026-11-17T09:30").unwrap());
    view()
        .child(
            segmented_control()
                .items(vec![
                    SegmentedControlItem::new("Light").icon("sun.max"),
                    SegmentedControlItem::new("Dark").icon("moon"),
                ])
                .selected_index(move || appearance.get())
                .on_change(move |index: &u32| appearance.set(*index)),
        )
        .child(
            switch()
                .accessibility_label(Some("Wi-Fi".into()))
                .checked(move || wifi.get())
                .on_change(move |checked: &bool| wifi.set(*checked)),
        )
        .child(
            date_time_picker()
                .mode(DateTimePickerMode::DateTime)
                .value(move || Some(at.get()))
                .minute_interval(5)
                .on_change(move |value: &DateTimePickerValue| at.set(*value)),
        )
}
```

`DateTimePickerValue` is made from `chrono`'s `NaiveDate`, `NaiveTime` and `NaiveDateTime` (`.into()`), or parsed from an ISO string.

## SegmentedControl

Picks one of a few options in place. iOS only.

| Prop | Rust | Type | Default | Description |
|---|---|---|---|---|
| `items` | `.items(..)` | `SegmentedControlItem[]` | `[]` | The segments, in order. |
| `selectedIndex` | `.selected_index(..)` | `number` | `0` | The selected segment; an index past the segments selects none. |
| `disabled` | `.disabled(..)` | `boolean` | `false` | No segment can be chosen: the system dims the control. |
| `onChange` | `.on_change(\|index: &u32\| ..)` | `({ index }) => void` | | The user chose another segment. The control shows the choice at once; the app sets `selectedIndex`. |

A `SegmentedControlItem` has a `title` (what the segment shows and what VoiceOver says), an optional `icon` (an [SF Symbol](https://developer.apple.com/sf-symbols/) shown in place of the title) and `disabled`. In Rust: `SegmentedControlItem::new(title).icon(name).disabled(true)`.

New `items` change the segments in place: the same number of segments keeps the control as it is, so a renamed or disabled segment does not flash. A width the style gives is shared by the segments.

## Switch

Turns one setting on or off. iOS and Android.

| Prop | Rust | Type | Default | Description |
|---|---|---|---|---|
| `checked` | `.checked(..)` | `boolean` | `false` | On or off. |
| `disabled` | `.disabled(..)` | `boolean` | `false` | It cannot be flipped: the system dims it. |
| `accessibilityLabel` | `.accessibility_label(..)` | `string` | none | What screen readers say with its state: the setting it turns on. |
| `onChange` | `.on_change(\|checked: &bool\| ..)` | `({ checked }) => void` | | The user flipped the switch: the value it asks for. The switch moves at once; the app sets `checked`. |

A value the app keeps (it turns the flip down) moves the switch back. It is on in its style's `accentColor` (`accent-*`), else in the theme's `primary`, else in the system's color. On Android it follows the app's light or dark appearance, in the system's dynamic colors on Android 12 and later (Material's baseline colors before).

## DateTimePicker

Sets a day, a time of day or both. iOS and Android.

| Prop | Rust | Type | Default | Description |
|---|---|---|---|---|
| `mode` | `.mode(..)` | `"date" \| "time" \| "dateTime"` | `"date"` | What it picks. |
| `value` | `.value(..)` | `string \| null` | `null` | The value shown, in the mode's form; `null` shows the present. |
| `min` | `.min(..)` | `string \| null` | `null` | The earliest value the user can pick. |
| `max` | `.max(..)` | `string \| null` | `null` | The latest value the user can pick. |
| `minuteInterval` | `.minute_interval(..)` | `number` | `1` | The minutes offered are this many apart: 1 to 30, a divisor of 60. Other numbers log a warning and step by 1. |
| `disabled` | `.disabled(..)` | `boolean` | `false` | It cannot be changed: the system dims it. |
| `accessibilityLabel` | `.accessibility_label(..)` | `string` | none | What screen readers say with its value: what it sets. |
| `onChange` | `.on_change(\|value: &DateTimePickerValue\| ..)` | `({ value }) => void` | | The user picked a value, in the mode's form. The picker shows it at once; the app sets `value`. |

Values are ISO strings in the mode's form, as HTML's date and time inputs take them: `"2026-11-17"` for `date`, `"09:30"` for `time`, `"2026-11-17T09:30"` for `dateTime`. They are wall-clock times: the picker shows them as they are, and the app ties them to the time zone it means. A value, `min` or `max` in another mode's form is left out with a warning.

- **iOS**: with `dateTime` the day and the time are two capsules. The calendar and the time wheels are the system's, in the user's locale, calendar and clock.
- **Android**: the value shows in the formats of the app's language (including one `setLocale` picks) and the user's 12- or 24-hour clock, in a field per part; a press opens the system's date or time dialog in the same language. The time dialog offers every minute: a pick between two steps of `minuteInterval` takes the step before it, and a pick past `min` or `max` takes the bound.

## Moving from NSH core

NSH up to 0.7 had these controls built in, in `nsh/mobile` (TSX) and `nsh_core::mobile` (Rust). They have moved here; the props, events and types are the same. To move:

1. `nsh add nsh-system-controls`.
2. Change the imports:

```diff
-import { DateTimePicker, SegmentedControl, Switch } from "nsh/mobile";
+import { DateTimePicker, SegmentedControl, Switch } from "nsh-system-controls";
```

```diff
-use nsh_core::mobile::{DateTimePickerValue, SegmentedControlItem, date_time_picker, segmented_control, switch};
+use nsh_system_controls::{DateTimePickerValue, SegmentedControlItem, date_time_picker, segmented_control, switch};
```

The rest of `nsh/mobile` (`TabBar`, `Haptics`, `AppSettings`, `BackHandler`, `InputAccessoryView`, `isAvailable`) stays in NSH.

The native components are named `nsh-system-controls/segmented-control`, `nsh-system-controls/switch` and `nsh-system-controls/date-time-picker` (`SEGMENTED_CONTROL_COMPONENT` and the others in Rust); names under `nsh/` are NSH's own.

## Developing

`cargo test` runs the controls headless: what each element sends its native component and how the user's choices come back. Check the platform code on its targets with `cargo clippy --all-targets --target aarch64-apple-ios` and `--target aarch64-linux-android`.

The Android controls are Java (`android/java`), packaged from `android/nsh-system-controls.jar`. After changing the Java, run `android/build.sh` (a JDK 17 or later and the Android SDK's platform `android-35`) and commit the jar with the sources; CI checks that they match.

## License

Apache-2.0, as NSH.
