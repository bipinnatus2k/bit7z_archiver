use std::any::TypeId;
use std::collections::HashMap;

use gpui::{App, SharedString};
use gpui_component::setting::*;
use gpui_component::text::Text;

/// A helper that turns typed get/set closures into SharedString-based ones.
/// The factory closure receives string-based get/set and can convert back.
pub struct SettingFieldRenderer {
    _private: (),
}

impl SettingFieldRenderer {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn init(_cx: &mut App) {
        // No global state needed — we use direct helper functions instead
    }
}

impl gpui::Global for SettingFieldRenderer {}

// ---------------------------------------------------------------------------
// Typed → SharedString bridge helpers
// ---------------------------------------------------------------------------

/// Convert a typed get/set pair into a SharedString-based dropdown field.
/// `to_str` converts T → &'static str (the value sent to the dropdown).
/// `from_str` converts &str → T (the value received from the dropdown).
pub fn dropdown_for<T>(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    options: Vec<(SharedString, SharedString)>,
    to_str: impl Fn(&T) -> &'static str + 'static,
    from_str: impl Fn(&str) -> T + 'static,
    get: impl Fn(&App) -> T + 'static,
    set: impl Fn(T, &mut App) + 'static,
) -> SettingItem {
    let g = move |cx: &App| -> SharedString { to_str(&get(cx)).into() };
    let s = move |val: SharedString, cx: &mut App| set(from_str(val.as_str()), cx);
    SettingItem::new(title, SettingField::dropdown(options, g, s)).description(description)
}

/// Convert a typed get/set pair into a bool Switch field.
pub fn switch_for<T>(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    check: impl Fn(&T) -> bool + 'static,
    get: impl Fn(&App) -> T + 'static,
    set: impl Fn(T, &mut App) + 'static,
) -> SettingItem {
    let g = move |cx: &App| check(&get(cx));
    let s = move |val: bool, cx: &mut App| {
        // For switches, we need to reconstruct T from the toggled state.
        // This only works when T is already bool.
    };
    SettingItem::new(title, SettingField::switch(g, s)).description(description)
}

/// Create a switch field for bool settings.
pub fn bool_switch(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    get: impl Fn(&App) -> bool + 'static,
    set: impl Fn(bool, &mut App) + 'static,
) -> SettingItem {
    SettingItem::new(title, SettingField::switch(get, set)).description(description)
}

/// Create a number field for f64 settings.
pub fn number_input(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    min: f64,
    max: f64,
    step: f64,
    get: impl Fn(&App) -> f64 + 'static,
    set: impl Fn(f64, &mut App) + 'static,
) -> SettingItem {
    let opts = NumberFieldOptions { min, max, step };
    SettingItem::new(title, SettingField::number_input(opts, get, set)).description(description)
}

/// Create a text input field for SharedString settings.
pub fn text_input(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    get: impl Fn(&App) -> SharedString + 'static,
    set: impl Fn(SharedString, &mut App) + 'static,
) -> SettingItem {
    SettingItem::new(title, SettingField::input(get, set)).description(description)
}

/// Create a dropdown field from string pairs.
pub fn string_dropdown(
    title: impl Into<SharedString>,
    description: impl Into<Text>,
    options: Vec<(SharedString, SharedString)>,
    get: impl Fn(&App) -> SharedString + 'static,
    set: impl Fn(SharedString, &mut App) + 'static,
) -> SettingItem {
    SettingItem::new(title, SettingField::dropdown(options, get, set)).description(description)
}
