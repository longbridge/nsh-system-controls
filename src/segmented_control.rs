//! [`SegmentedControl`]: the system's segmented control, iOS's.

use nsh_core::{
    JsonValue,
    element::{NativeEvent, native},
    platform::Os,
    prelude::*,
};

use crate::SEGMENTED_CONTROL_COMPONENT;

/// One segment of a [`SegmentedControl`].
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct SegmentedControlItem {
    /// Its title: what the segment shows and what VoiceOver says (with an
    /// icon, only what VoiceOver says).
    pub title: SharedString,
    /// Its icon, an SF Symbol's name (`"sun.max"`), shown in place of the
    /// title.
    #[prop(skip_none)]
    pub icon: Option<SharedString>,
    /// The user cannot choose it (default false): the system dims it.
    #[prop(default)]
    pub disabled: bool,
}

impl SegmentedControlItem {
    /// A segment titled `title`, with no icon.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Its icon, an SF Symbol's name ([`SegmentedControlItem::icon`]).
    pub fn icon(mut self, name: impl Into<SharedString>) -> Self {
        self.icon = Some(name.into());
        self
    }

    /// Whether the user cannot choose it
    /// ([`SegmentedControlItem::disabled`]).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The props of [`SegmentedControl`].
#[derive(Debug, Clone, PartialEq, Props)]
pub struct SegmentedControlProps {
    /// The segments, in order.
    #[prop(default)]
    pub items: Prop<Vec<SegmentedControlItem>>,
    /// The selected segment, by index (default 0; one past the segments
    /// selects none). The app owns it: the user's choice is reported with
    /// `onChange`.
    #[prop(default)]
    pub selected_index: Prop<u32>,
    /// No segment can be chosen (default false): the system dims the
    /// control and VoiceOver says it is dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// The events of a [`SegmentedControl`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum SegmentedControlEvent {
    /// The user chose another segment: its index. The control shows the
    /// choice at once, as the system's does; the app sets
    /// `selectedIndex`.
    #[args(index)]
    Change(u32),
}

/// The system's segmented control: a `UISegmentedControl` on iOS (Liquid
/// Glass on iOS 26 and later, the system's earlier look before), in the
/// system's look, behavior, VoiceOver and keyboard focus. The app gives
/// the segments (`items`), says which is selected (`selectedIndex`), hears
/// the user's choice (`onChange`) and places the control with its `class`
/// and `style`; its size is the system's where the style leaves it open (a
/// width the style gives is shared by the segments). New `items` change
/// the segments in place, and the selection follows `selectedIndex`. Other
/// platforms have no such control: there it takes no room and shows
/// nothing, and the app shows its choice with `nsh/base`'s `ToggleGroup` or
/// `RadioGroup`.
#[component]
pub fn SegmentedControl(
    props: SegmentedControlProps,
    cx: Scope<SegmentedControlEvent>,
) -> impl IntoElement {
    if os() != Os::Ios {
        return view().hidden().into_element();
    }
    let items = props.items;
    native(SEGMENTED_CONTROL_COMPONENT)
        .prop("items", move || items.with(|items| segments_json(items)))
        .prop("selectedIndex", move || {
            JsonValue::from(props.selected_index.get())
        })
        .prop("disabled", move || JsonValue::Bool(props.disabled.get()))
        .on_native_event(move |event: &NativeEvent| {
            if &*event.name == "change"
                && let Some(index) = event.data.as_u64()
            {
                cx.emit(SegmentedControlEvent::Change(index as u32));
            }
        })
        .into_element()
}

/// The segments as the platform reads them: `[{ title, icon?, disabled? }]`.
fn segments_json(items: &[SegmentedControlItem]) -> JsonValue {
    items
        .iter()
        .map(|item| {
            let mut segment = serde_json::Map::new();
            segment.insert("title".into(), item.title.as_ref().into());
            if let Some(icon) = &item.icon {
                segment.insert("icon".into(), icon.as_ref().into());
            }
            if item.disabled {
                segment.insert("disabled".into(), true.into());
            }
            JsonValue::Object(segment)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{SegmentedControlItem, segments_json};

    #[test]
    fn segments_reach_the_platform_as_titles_icons_and_disabled() {
        let segments = [
            SegmentedControlItem::new("Light").icon("sun.max"),
            SegmentedControlItem::new("Dark").disabled(true),
        ];
        assert_eq!(
            segments_json(&segments),
            json!([
                { "title": "Light", "icon": "sun.max" },
                { "title": "Dark", "disabled": true },
            ])
        );
    }
}
