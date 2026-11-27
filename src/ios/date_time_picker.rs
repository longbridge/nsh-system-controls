//! [`crate::DateTimePicker`] on iOS: a `UIDatePicker` in its compact
//! style, the system's capsule of the value that opens its calendar or time
//! wheels. The app places it and owns the value; the component reports the
//! picker's own size for what the style leaves open and the user's pick as
//! `change` with its fields (`{ year, month, day, hour, minute }`).
//!
//! The values are wall-clock times: they become dates in the user's
//! calendar and time zone, which the picker shows them in, and its dates
//! come back as the same fields.
//!
//! The picker shows its words and formats in the user's locale
//! (`NSLocale.currentLocale`, with their region settings) unless the app's
//! text is in another language (`locale`): then in that one's, with the
//! user's 12- or 24-hour clock, and its weeks start on that language's
//! first day.

use std::{ptr::NonNull, rc::Rc};

use block2::RcBlock;
use nsh_core::JsonValue;
use objc2::{
    MainThreadMarker, MainThreadOnly, msg_send,
    rc::{Retained, Weak},
};
use objc2_core_foundation::{CGFloat, CGRect, CGSize};
use objc2_foundation::{
    NSCalendar, NSCalendarUnit, NSDate, NSDateComponents, NSDateFormatter, NSLocale,
    NSLocaleCalendar, NSLocaleCountryCode, NSString, ns_string,
};
use objc2_ui_kit::{
    UIAccessibilityContainerType, UIAction, UIControlEvents, UIDatePicker, UIDatePickerMode,
    UIDatePickerStyle, UIView,
};

use nsh_platform_ios::{ComponentContext, ComponentFactory, NativeComponent};

/// The picker's mode for a `mode` prop (`"date"` by default).
fn picker_mode(mode: Option<&str>) -> UIDatePickerMode {
    match mode {
        Some("time") => UIDatePickerMode::Time,
        Some("dateTime") => UIDatePickerMode::DateAndTime,
        _ => UIDatePickerMode::Date,
    }
}

/// The value's fields that are there: a day, a time of day or both.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Fields {
    date: Option<(isize, isize, isize)>,
    time: Option<(isize, isize)>,
}

/// The fields of a value prop (`{ year, month, day }`, `{ hour, minute }`
/// or all five); `None` for none.
fn parse_fields(value: Option<&JsonValue>) -> Option<Fields> {
    let value = value?.as_object()?;
    let field = |name: &str| {
        value
            .get(name)
            .and_then(JsonValue::as_i64)
            .map(|v| v as isize)
    };
    let date = (|| Some((field("year")?, field("month")?, field("day")?)))();
    let time = (|| Some((field("hour")?, field("minute")?)))();
    (date.is_some() || time.is_some()).then_some(Fields { date, time })
}

/// The date `fields` are in the user's calendar: a time of day falls on
/// today, a day alone at noon (midnight is missing on days a time zone
/// starts its summer time at it).
fn date_of(calendar: &NSCalendar, fields: Fields) -> Option<Retained<NSDate>> {
    let components = calendar.components_fromDate(
        NSCalendarUnit::Year | NSCalendarUnit::Month | NSCalendarUnit::Day,
        &NSDate::now(),
    );
    if let Some((year, month, day)) = fields.date {
        components.setYear(year);
        components.setMonth(month);
        components.setDay(day);
    }
    let (hour, minute) = fields.time.unwrap_or((12, 0));
    components.setHour(hour);
    components.setMinute(minute);
    calendar.dateFromComponents(&components)
}

/// The fields of `date` in the user's calendar.
fn fields_of(calendar: &NSCalendar, date: &NSDate) -> JsonValue {
    let c: Retained<NSDateComponents> = calendar.components_fromDate(
        NSCalendarUnit::Year
            | NSCalendarUnit::Month
            | NSCalendarUnit::Day
            | NSCalendarUnit::Hour
            | NSCalendarUnit::Minute,
        date,
    );
    serde_json::json!({
        "year": c.year(),
        "month": c.month(),
        "day": c.day(),
        "hour": c.hour(),
        "minute": c.minute(),
    })
}

/// The ICU `hours` keyword (`h12`, `h23`, …) of a time format's hour
/// symbol (`h:mm a`, `HH:mm`); `None` without one. Quoted text is not a
/// symbol.
fn hour_cycle_of_pattern(pattern: &str) -> Option<&'static str> {
    let mut quoted = false;
    for c in pattern.chars() {
        match c {
            '\'' => quoted = !quoted,
            'h' if !quoted => return Some("h12"),
            'H' if !quoted => return Some("h23"),
            'K' if !quoted => return Some("h11"),
            'k' if !quoted => return Some("h24"),
            _ => {}
        }
    }
    None
}

/// `identifier` (a canonical locale identifier, `de_DE`) with the ICU
/// `hours` keyword `hours`, unless it has one.
fn with_hour_cycle(identifier: &str, hours: Option<&str>) -> String {
    match hours {
        Some(_) if identifier.contains("hours=") => identifier.to_owned(),
        Some(hours) if identifier.contains('@') => format!("{identifier};hours={hours}"),
        Some(hours) => format!("{identifier}@hours={hours}"),
        None => identifier.to_owned(),
    }
}

/// The script text in `language` is written in: `script` where the locale
/// names one, else Chinese's by its region (Traditional in Taiwan, Hong
/// Kong and Macau, Simplified elsewhere); `None` for a language that does
/// not tell.
fn script_of(language: &str, script: Option<&str>, region: Option<&str>) -> Option<String> {
    if let Some(script) = script {
        return Some(script.to_owned());
    }
    if !language.eq_ignore_ascii_case("zh") {
        return None;
    }
    let traditional = region.is_some_and(|region| {
        ["TW", "HK", "MO"]
            .iter()
            .any(|r| r.eq_ignore_ascii_case(region))
    });
    Some(if traditional { "Hant" } else { "Hans" }.to_owned())
}

/// Whether locales of these language and script codes are in the same
/// language: a script only tells them apart where both name one.
fn same_language(
    (language, script): (&str, Option<&str>),
    (other, other_script): (&str, Option<&str>),
) -> bool {
    language.eq_ignore_ascii_case(other)
        && match (script, other_script) {
            (Some(script), Some(other)) => script.eq_ignore_ascii_case(other),
            _ => true,
        }
}

/// The language code of `locale` and the script it is written in
/// ([`script_of`]).
fn language_of(locale: &NSLocale) -> (String, Option<String>) {
    let language = locale.languageCode().to_string();
    let script = locale.scriptCode().map(|script| script.to_string());
    // SAFETY: a key Foundation defines; its value is the region's code.
    let region = locale
        .objectForKey(unsafe { NSLocaleCountryCode })
        .and_then(|code| code.downcast::<NSString>().ok())
        .map(|code| code.to_string());
    let script = script_of(&language, script.as_deref(), region.as_deref());
    (language, script)
}

/// The locale the picker shows the app's text language `tag` (BCP 47) in:
/// `None`, the user's own (`currentLocale`, with the region settings
/// Settings gives it), where `tag` is none or in the user's language;
/// else `tag`'s, with the user's 12- or 24-hour clock.
fn picker_locale(tag: Option<&str>) -> Option<Retained<NSLocale>> {
    let tag = tag.filter(|tag| !tag.is_empty())?;
    let identifier = NSLocale::canonicalLocaleIdentifierFromString(&NSString::from_str(tag));
    let shown = NSLocale::localeWithLocaleIdentifier(&identifier);
    let current = NSLocale::currentLocale();
    let (language, script) = language_of(&shown);
    let (user_language, user_script) = language_of(&current);
    if same_language(
        (&language, script.as_deref()),
        (&user_language, user_script.as_deref()),
    ) {
        return None;
    }
    // The hours of the user's time format, which 24-hour time changes.
    let hours =
        NSDateFormatter::dateFormatFromTemplate_options_locale(ns_string!("j"), 0, Some(&current))
            .and_then(|format| hour_cycle_of_pattern(&format.to_string()));
    let identifier = with_hour_cycle(&identifier.to_string(), hours);
    Some(NSLocale::localeWithLocaleIdentifier(&NSString::from_str(
        &identifier,
    )))
}

/// The user's calendar (their calendar system and time zone) in `locale`,
/// with the weeks starting on `locale`'s first day.
fn calendar_in(locale: &NSLocale) -> Option<Retained<NSCalendar>> {
    let user = NSCalendar::currentCalendar();
    let calendar = NSCalendar::calendarWithIdentifier(&user.calendarIdentifier())?;
    calendar.setTimeZone(&user.timeZone());
    calendar.setLocale(Some(locale));
    // SAFETY: a key Foundation defines; its value is the locale's calendar.
    let own = locale.objectForKey(unsafe { NSLocaleCalendar });
    if let Some(own) = own.and_then(|own| own.downcast::<NSCalendar>().ok()) {
        calendar.setFirstWeekday(own.firstWeekday());
    }
    Some(calendar)
}

struct SystemDateTimePicker {
    picker: Retained<UIDatePicker>,
    context: ComponentContext,
}

impl SystemDateTimePicker {
    fn date(&self, value: Option<&JsonValue>) -> Option<Retained<NSDate>> {
        date_of(&NSCalendar::currentCalendar(), parse_fields(value)?)
    }

    /// The picker's own size, for the dimensions the style leaves open: a
    /// compact picker's changes with its mode and with the value it shows.
    fn report_size(&self) {
        let size = self.picker.sizeThatFits(CGSize::new(0.0, 0.0));
        self.context.set_size(size.width as f32, size.height as f32);
    }

    /// Shows the picker in `locale` (`None`: the user's) and reports its
    /// size for the new formats. A compact picker fits its capsules' text
    /// to its width, a shorter date where it is short, so it is laid out
    /// and measured at a width any date fits: a language with longer dates
    /// is not left in the short form the last one's width chose.
    fn show_locale(&self, locale: Option<&NSLocale>) {
        let calendar = locale.and_then(calendar_in);
        // Nil returns each to the user's.
        self.picker.setLocale(locale);
        self.picker.setCalendar(calendar.as_deref());
        let frame = self.picker.frame();
        let mut wide = frame;
        wide.size.width = wide.size.width.max(MEASURING_WIDTH);
        self.picker.setFrame(wide);
        self.picker.setNeedsLayout();
        self.picker.layoutIfNeeded();
        let size = self.picker.sizeThatFits(CGSize::new(0.0, 0.0));
        self.picker.setFrame(frame);
        self.picker.setNeedsLayout();
        self.context.set_size(size.width as f32, size.height as f32);
    }
}

/// A width wider than a compact picker's capsules in any language.
const MEASURING_WIDTH: CGFloat = 2000.0;

impl NativeComponent for SystemDateTimePicker {
    fn view(&self) -> &UIView {
        &self.picker
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match name {
            "mode" => {
                self.picker
                    .setDatePickerMode(picker_mode(value.and_then(JsonValue::as_str)));
                self.report_size();
            }
            "value" => {
                // None shows the present, as a new picker does.
                let date = self.date(value).unwrap_or_else(NSDate::now);
                self.picker.setDate_animated(&date, false);
                self.report_size();
            }
            "min" => self.picker.setMinimumDate(self.date(value).as_deref()),
            "max" => self.picker.setMaximumDate(self.date(value).as_deref()),
            "locale" => {
                let locale = picker_locale(value.and_then(JsonValue::as_str));
                self.show_locale(locale.as_deref());
            }
            "minuteInterval" => {
                let interval = value.and_then(JsonValue::as_i64).unwrap_or(1);
                self.picker.setMinuteInterval(interval as isize);
            }
            "disabled" => {
                let disabled = value.and_then(JsonValue::as_bool).unwrap_or(false);
                self.picker.setEnabled(!disabled);
            }
            "accessibilityLabel" => {
                let label = value.and_then(JsonValue::as_str).map(NSString::from_str);
                // VoiceOver moves to the capsules inside a compact picker, so
                // the picker names them as a group, read as VoiceOver enters it.
                let group = if label.is_some() {
                    UIAccessibilityContainerType::SemanticGroup
                } else {
                    UIAccessibilityContainerType::None
                };
                // SAFETY: `accessibilityLabel` and `accessibilityContainerType`
                // are UIAccessibility's properties, which every NSObject has;
                // a nil label removes it.
                unsafe {
                    let () = msg_send![&*self.picker, setAccessibilityLabel: label.as_deref()];
                    let () = msg_send![&*self.picker, setAccessibilityContainerType: group];
                }
            }
            _ => {}
        }
    }
}

/// Makes the picker of each `DateTimePicker` element
/// ([`crate::DATE_TIME_PICKER_COMPONENT`]).
pub(crate) fn factory() -> ComponentFactory {
    Rc::new(|context: ComponentContext| {
        let mtm = MainThreadMarker::new().expect("the date picker is made on the main thread");
        let picker = UIDatePicker::initWithFrame(UIDatePicker::alloc(mtm), CGRect::default());
        picker.setPreferredDatePickerStyle(UIDatePickerStyle::Compact);
        picker.setDatePickerMode(UIDatePickerMode::Date);
        // The user's pick, read from the picker the action came from: a weak
        // reference, as the picker keeps the action and its block.
        let weak = Weak::from_retained(&picker);
        let emit = context.clone();
        let changed = RcBlock::new(move |_: NonNull<UIAction>| {
            if let Some(picker) = weak.load() {
                emit.emit(
                    "change",
                    fields_of(&NSCalendar::currentCalendar(), &picker.date()),
                );
                let size = picker.sizeThatFits(CGSize::new(0.0, 0.0));
                emit.set_size(size.width as f32, size.height as f32);
            }
        });
        // SAFETY: the handler is a block UIKit copies; it runs on the main
        // thread, as the picker's events do.
        let action = unsafe { UIAction::actionWithHandler(RcBlock::as_ptr(&changed), mtm) };
        picker.addAction_forControlEvents(&action, UIControlEvents::ValueChanged);
        let component = SystemDateTimePicker { picker, context };
        component.report_size();
        Box::new(component) as Box<dyn NativeComponent>
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Fields, hour_cycle_of_pattern, parse_fields, same_language, script_of, with_hour_cycle,
    };

    #[test]
    fn the_hour_cycle_is_the_formats_hour_symbol() {
        assert_eq!(hour_cycle_of_pattern("h:mm a"), Some("h12"));
        assert_eq!(hour_cycle_of_pattern("HH:mm"), Some("h23"));
        assert_eq!(hour_cycle_of_pattern("'h' HH"), Some("h23"));
        assert_eq!(hour_cycle_of_pattern("K:mm a"), Some("h11"));
        assert_eq!(hour_cycle_of_pattern("mm"), None);
    }

    #[test]
    fn the_users_hour_cycle_joins_the_locale() {
        assert_eq!(with_hour_cycle("de_DE", Some("h12")), "de_DE@hours=h12");
        assert_eq!(
            with_hour_cycle("de_DE@calendar=gregorian", Some("h12")),
            "de_DE@calendar=gregorian;hours=h12"
        );
        assert_eq!(
            with_hour_cycle("de_DE@hours=h23", Some("h12")),
            "de_DE@hours=h23"
        );
        assert_eq!(with_hour_cycle("de_DE", None), "de_DE");
    }

    #[test]
    fn locales_of_one_language_are_the_same_language() {
        assert!(same_language(("en", None), ("en", None)));
        assert!(same_language(("zh", Some("Hant")), ("zh", None)));
        assert!(!same_language(("de", None), ("en", None)));
        assert!(!same_language(("zh", Some("Hans")), ("zh", Some("Hant"))));
    }

    #[test]
    fn chinese_is_written_in_its_regions_script() {
        let script = |language, script, region| script_of(language, script, region);
        assert_eq!(script("zh", None, Some("TW")).as_deref(), Some("Hant"));
        assert_eq!(script("zh", None, Some("HK")).as_deref(), Some("Hant"));
        assert_eq!(script("zh", None, Some("MO")).as_deref(), Some("Hant"));
        assert_eq!(script("zh", None, Some("CN")).as_deref(), Some("Hans"));
        assert_eq!(script("zh", None, Some("SG")).as_deref(), Some("Hans"));
        assert_eq!(script("zh", None, None).as_deref(), Some("Hans"));
        assert_eq!(
            script("zh", Some("Hans"), Some("TW")).as_deref(),
            Some("Hans")
        );
        assert_eq!(script("en", None, Some("US")), None);
        assert_eq!(script("sr", Some("Latn"), None).as_deref(), Some("Latn"));
    }

    #[test]
    fn a_chinese_script_the_app_picks_is_not_the_users_other_one() {
        // The user's `zh_TW` against the app's `zh-Hans`: two scripts.
        let user = ("zh", script_of("zh", None, Some("TW")));
        let app = ("zh", script_of("zh", Some("Hans"), None));
        assert!(!same_language(
            (app.0, app.1.as_deref()),
            (user.0, user.1.as_deref())
        ));
        // The user's `zh_CN` against the app's `zh`: one.
        let user = ("zh", script_of("zh", None, Some("CN")));
        let app = ("zh", script_of("zh", None, None));
        assert!(same_language(
            (app.0, app.1.as_deref()),
            (user.0, user.1.as_deref())
        ));
    }

    #[test]
    fn values_carry_the_fields_they_have() {
        assert_eq!(
            parse_fields(Some(&json!({ "year": 2026, "month": 11, "day": 17 }))),
            Some(Fields {
                date: Some((2026, 11, 17)),
                time: None,
            })
        );
        assert_eq!(
            parse_fields(Some(&json!({ "hour": 9, "minute": 30 }))),
            Some(Fields {
                date: None,
                time: Some((9, 30)),
            })
        );
        assert_eq!(parse_fields(Some(&json!(null))), None);
        assert_eq!(parse_fields(Some(&json!({ "hour": 9 }))), None);
        assert_eq!(parse_fields(None), None);
    }
}
