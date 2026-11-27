//! [`DateTimePicker`]: the system's date and time picker, iOS's and
//! Android's.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use nsh_core::{
    JsonValue,
    element::{JsHost, JsIn, JsOwned, JsValue, NativeEvent, PropError, TsTypes, native},
    platform::Os,
    prelude::*,
};

use crate::DATE_TIME_PICKER_COMPONENT;

/// What a [`DateTimePicker`] picks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum DateTimePickerMode {
    /// A day (default).
    #[default]
    Date,
    /// A time of day, in hours and minutes.
    Time,
    /// A day and a time of day.
    DateTime,
}

/// A [`DateTimePicker`]'s value: a day, a time of day or both, as the
/// picker shows them (a wall-clock time, in no time zone: the app gives
/// the one it means). TS: an ISO string, as HTML's date and time inputs
/// take it: `"2026-11-17"`, `"09:30"`, `"2026-11-17T09:30"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimePickerValue {
    /// A day, `"2026-11-17"` (the `date` mode's).
    Date(NaiveDate),
    /// A time of day, `"09:30"` (the `time` mode's).
    Time(NaiveTime),
    /// A day and a time of day, `"2026-11-17T09:30"` (the `dateTime`
    /// mode's).
    DateTime(NaiveDateTime),
}

impl From<NaiveDate> for DateTimePickerValue {
    fn from(value: NaiveDate) -> Self {
        Self::Date(value)
    }
}

impl From<NaiveTime> for DateTimePickerValue {
    fn from(value: NaiveTime) -> Self {
        Self::Time(value)
    }
}

impl From<NaiveDateTime> for DateTimePickerValue {
    fn from(value: NaiveDateTime) -> Self {
        Self::DateTime(value)
    }
}

impl DateTimePickerValue {
    /// The value an ISO string (`"2026-11-17"`, `"09:30"`,
    /// `"2026-11-17T09:30"`) gives; `None` for anything else, seconds
    /// included: the pickers show minutes.
    pub fn parse(text: &str) -> Option<Self> {
        if let Ok(v) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M") {
            Some(Self::DateTime(v))
        } else if let Ok(v) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
            Some(Self::Date(v))
        } else {
            NaiveTime::parse_from_str(text, "%H:%M")
                .ok()
                .map(Self::Time)
        }
    }

    /// The value as an ISO string.
    pub fn iso(&self) -> String {
        match self {
            Self::Date(v) => v.format("%Y-%m-%d").to_string(),
            Self::Time(v) => v.format("%H:%M").to_string(),
            Self::DateTime(v) => v.format("%Y-%m-%dT%H:%M").to_string(),
        }
    }

    /// The mode whose values it is.
    pub fn mode(&self) -> DateTimePickerMode {
        match self {
            Self::Date(_) => DateTimePickerMode::Date,
            Self::Time(_) => DateTimePickerMode::Time,
            Self::DateTime(_) => DateTimePickerMode::DateTime,
        }
    }

    /// The value as the platforms read it: its fields
    /// (`{ year, month, day }`, `{ hour, minute }` or all five).
    fn fields(&self) -> JsonValue {
        let mut fields = serde_json::Map::new();
        let date = match self {
            Self::Date(d) => Some(*d),
            Self::DateTime(v) => Some(v.date()),
            Self::Time(_) => None,
        };
        let time = match self {
            Self::Time(t) => Some(*t),
            Self::DateTime(v) => Some(v.time()),
            Self::Date(_) => None,
        };
        if let Some(date) = date {
            fields.insert("year".into(), date.year().into());
            fields.insert("month".into(), date.month().into());
            fields.insert("day".into(), date.day().into());
        }
        if let Some(time) = time {
            fields.insert("hour".into(), time.hour().into());
            fields.insert("minute".into(), time.minute().into());
        }
        JsonValue::Object(fields)
    }

    /// The value of `mode` a platform's `{ year, month, day, hour, minute }`
    /// gives; `None` when a field is missing or out of range.
    fn of_fields(mode: DateTimePickerMode, fields: &JsonValue) -> Option<Self> {
        let field = |name: &str| -> Option<u32> { fields.get(name)?.as_u64()?.try_into().ok() };
        let date = || {
            NaiveDate::from_ymd_opt(
                field("year")?.try_into().ok()?,
                field("month")?,
                field("day")?,
            )
        };
        let time = || NaiveTime::from_hms_opt(field("hour")?, field("minute")?, 0);
        Some(match mode {
            DateTimePickerMode::Date => Self::Date(date()?),
            DateTimePickerMode::Time => Self::Time(time()?),
            DateTimePickerMode::DateTime => Self::DateTime(date()?.and_time(time()?)),
        })
    }
}

impl JsValue for DateTimePickerValue {
    fn ts_type(_types: &mut TsTypes) -> String {
        "string".into()
    }

    fn from_js(value: JsIn<'_>) -> Result<Self, PropError> {
        value
            .as_string()
            .and_then(|s| Self::parse(&s))
            .ok_or_else(|| {
                value.expected(
                    "an ISO date or time (`\"2026-11-17\"`, `\"09:30\"`, `\"2026-11-17T09:30\"`)",
                )
            })
    }

    fn to_js<'a>(&self, host: &'a dyn JsHost) -> Result<JsOwned<'a>, PropError> {
        SharedString::from(self.iso()).to_js(host)
    }
}

/// The props of [`DateTimePicker`].
#[derive(Debug, Clone, PartialEq, Props)]
pub struct DateTimePickerProps {
    /// What it picks: a day (default), a time of day or both.
    #[prop(default)]
    pub mode: Prop<DateTimePickerMode>,
    /// The value shown, in the mode's form (`"2026-11-17"`, `"09:30"`,
    /// `"2026-11-17T09:30"`); none shows the present day and time, as the
    /// system's pickers do. The app owns it: the user's pick is reported
    /// with `onChange`.
    pub value: Prop<Option<DateTimePickerValue>>,
    /// The earliest value the user can pick, in the mode's form.
    pub min: Prop<Option<DateTimePickerValue>>,
    /// The latest value the user can pick, in the mode's form.
    pub max: Prop<Option<DateTimePickerValue>>,
    /// The minutes offered are this many apart (default 1): 1 to 30, a
    /// divisor of 60 (`5`, `15`, `30`).
    #[prop(default = 1)]
    pub minute_interval: Prop<u32>,
    /// It cannot be changed (default false): the system dims it.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// What screen readers say with its value: what it sets (`"Due
    /// date"`). The picker shows only the value, so a label beside it
    /// names it for sighted users alone.
    pub accessibility_label: Prop<Option<SharedString>>,
}

/// The events of a [`DateTimePicker`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum DateTimePickerEvent {
    /// The user picked a value, in the mode's form. The picker shows it at
    /// once, as the system's does; the app sets `value`.
    #[args(value)]
    Change(DateTimePickerValue),
}

/// The system's date and time picker, in the system's look, behavior and
/// accessibility: on iOS a `UIDatePicker` in its compact style (a value in
/// a capsule that opens the system's calendar or time wheels; a day and a
/// time are two), on Android the value in a field that opens the system's
/// date or time dialog. The app says what it picks (`mode`), gives the
/// value (`value`, a wall-clock time the app ties to its time zone), its
/// bounds (`min`, `max`) and the minutes' step (`minuteInterval`), names it
/// for screen readers (`accessibilityLabel`), hears the user's pick
/// (`onChange`) and places the picker with its `class` and `style`; its
/// size is the system's where the style leaves it open. The desktops and
/// the web have no such control: there it takes no room and shows
/// nothing, and the app picks with `nsh/base`'s `DatePicker` and
/// `TimeField`.
#[component]
pub fn DateTimePicker(
    props: DateTimePickerProps,
    cx: Scope<DateTimePickerEvent>,
) -> impl IntoElement {
    if !matches!(os(), Os::Ios | Os::Android) {
        return view().hidden().into_element();
    }
    let mode = props.mode;
    // A value of another mode's form is not shown as something else: the
    // picker shows the present, as with none.
    let fields = move |value: Option<DateTimePickerValue>, prop: &str| match value {
        Some(value) if value.mode() == mode.get() => value.fields(),
        Some(value) => {
            log::warn!(
                target: "js",
                "DateTimePicker: `{prop}` {:?} is not a value of mode `{}`",
                value.iso(),
                mode.get().js_name()
            );
            JsonValue::Null
        }
        None => JsonValue::Null,
    };
    let picker = native(DATE_TIME_PICKER_COMPONENT)
        .prop("mode", move || JsonValue::from(mode.get().js_name()))
        .prop("value", move || fields(props.value.get(), "value"))
        .prop("min", move || fields(props.min.get(), "min"))
        .prop("max", move || fields(props.max.get(), "max"))
        .prop("minuteInterval", move || {
            let interval = props.minute_interval.get();
            if (1..=30).contains(&interval) && 60 % interval == 0 {
                JsonValue::from(interval)
            } else {
                log::warn!(
                    target: "js",
                    "DateTimePicker: `minuteInterval` {interval} is not a divisor of 60 from 1 to 30; the minutes step by 1"
                );
                JsonValue::from(1)
            }
        })
        .prop("disabled", move || JsonValue::Bool(props.disabled.get()))
        .prop("accessibilityLabel", move || {
            props
                .accessibility_label
                .get()
                .map_or(JsonValue::Null, |label| label.as_ref().into())
        });
    crate::system_environment(picker, true)
        .on_native_event(move |event: &NativeEvent| {
            if &*event.name == "change"
                && let Some(value) =
                    DateTimePickerValue::of_fields(mode.get_untracked(), &event.data)
            {
                cx.emit(DateTimePickerEvent::Change(value));
            }
        })
        .into_element()
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};
    use serde_json::json;

    use super::{DateTimePickerMode, DateTimePickerValue};

    #[test]
    fn picker_values_are_iso_strings_of_minutes() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 17).unwrap();
        let time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        for (text, value) in [
            ("2026-11-17", DateTimePickerValue::Date(day)),
            ("09:30", DateTimePickerValue::Time(time)),
            (
                "2026-11-17T09:30",
                DateTimePickerValue::DateTime(day.and_time(time)),
            ),
        ] {
            assert_eq!(DateTimePickerValue::parse(text), Some(value), "{text}");
            assert_eq!(value.iso(), text);
        }
        for text in [
            "09:30:15",
            "2026-11-17T09:30:00",
            "2026-02-30",
            "25:00",
            "",
            "today",
        ] {
            assert_eq!(DateTimePickerValue::parse(text), None, "{text}");
        }
    }

    #[test]
    fn picker_values_reach_the_platform_as_fields_and_come_back_in_the_mode() {
        let value = DateTimePickerValue::parse("2026-11-17T09:30").unwrap();
        let fields = json!({ "year": 2026, "month": 11, "day": 17, "hour": 9, "minute": 30 });
        assert_eq!(value.fields(), fields);
        assert_eq!(
            DateTimePickerValue::parse("2026-11-17").unwrap().fields(),
            json!({ "year": 2026, "month": 11, "day": 17 })
        );
        assert_eq!(
            DateTimePickerValue::parse("09:30").unwrap().fields(),
            json!({ "hour": 9, "minute": 30 })
        );
        assert_eq!(
            DateTimePickerValue::of_fields(DateTimePickerMode::DateTime, &fields),
            Some(value)
        );
        assert_eq!(
            DateTimePickerValue::of_fields(DateTimePickerMode::Date, &fields),
            DateTimePickerValue::parse("2026-11-17")
        );
        assert_eq!(
            DateTimePickerValue::of_fields(DateTimePickerMode::Time, &fields),
            DateTimePickerValue::parse("09:30")
        );
        let bad = json!({ "year": 2026, "month": 2, "day": 30, "hour": 9, "minute": 30 });
        assert_eq!(
            DateTimePickerValue::of_fields(DateTimePickerMode::Date, &bad),
            None
        );
        assert_eq!(
            DateTimePickerValue::of_fields(DateTimePickerMode::Time, &json!({ "hour": 9 })),
            None
        );
    }
}
