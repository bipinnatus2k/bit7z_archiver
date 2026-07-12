use std::any::{Any, TypeId};

use gpui::*;
use gpui_component::button::Button;
use gpui_component::{h_flex, ActiveTheme, Sizable};

use crate::backend::store::SettingsStore;
use crate::notify_panel;
use bit7z_domain::preferences::Preferences;

// ── Type-erased setting field ─────────────────────────────────────────────

pub trait AnySettingField: Send + Sync {
    fn type_id(&self) -> TypeId;
    fn render_control(&self, id_prefix: &str, cx: &mut App) -> AnyElement;
    fn label(&self) -> SharedString;
    fn description(&self) -> SharedString;
}

// ── Concrete field that stores a render closure ──────────────────────────

pub struct SettingField {
    pub label: SharedString,
    pub description: SharedString,
    pub render: Box<dyn Fn(&str, &mut App) -> AnyElement + Send + Sync>,
    pub type_id: TypeId,
}

impl AnySettingField for SettingField {
    fn type_id(&self) -> TypeId {
        self.type_id
    }
    fn render_control(&self, id_prefix: &str, cx: &mut App) -> AnyElement {
        (self.render)(id_prefix, cx)
    }
    fn label(&self) -> SharedString {
        self.label.clone()
    }
    fn description(&self) -> SharedString {
        self.description.clone()
    }
}

// ── Factory functions ────────────────────────────────────────────────────

pub fn bool_field(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    read: fn(&Preferences) -> bool,
    write: fn(&mut Preferences, bool),
) -> SettingField {
    let label = label.into();
    let description = description.into();
    SettingField {
        label: label.clone(),
        description: description.clone(),
        type_id: TypeId::of::<bool>(),
        render: Box::new(move |id_prefix, cx| {
            let cur = read(&SettingsStore::get(cx).prefs);
            let id = SharedString::from(format!("{id_prefix}-bool"));
            let label = if cur { "[x]" } else { "[ ]" };
            let fg = cx.theme();
            Button::new(id)
                .label(label)
                .on_click(move |_, _, cx| {
                    let new_val = !read(&SettingsStore::get(cx).prefs);
                    write_setting(move |p| write(p, new_val), cx);
                })
                .into_any_element()
        }),
    }
}

pub fn f64_field(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    read: fn(&Preferences) -> f64,
    write: fn(&mut Preferences, f64),
    min: f64,
    max: f64,
) -> SettingField {
    let label = label.into();
    let description = description.into();
    SettingField {
        label: label.clone(),
        description: description.clone(),
        type_id: TypeId::of::<f64>(),
        render: Box::new(move |id_prefix, cx| {
            let cur = read(&SettingsStore::get(cx).prefs);
            let min_v = min;
            let max_v = max;
            let id = SharedString::from(format!("{id_prefix}-f64"));
            h_flex()
                .gap_1()
                .child(
                    Button::new(SharedString::from(format!("{id}-dec")))
                        .label("-")
                        .on_click(move |_, _, cx| {
                            let v = read(&SettingsStore::get(cx).prefs);
                            write_setting(move |p| write(p, (v - 1.0).max(min_v)), cx);
                        }),
                )
                .child(div().px_2().child(format!("{:.0}", cur)))
                .child(
                    Button::new(SharedString::from(format!("{id}-inc")))
                        .label("+")
                        .on_click(move |_, _, cx| {
                            let v = read(&SettingsStore::get(cx).prefs);
                            write_setting(move |p| write(p, (v + 1.0).min(max_v)), cx);
                        }),
                )
                .into_any_element()
        }),
    }
}

pub fn dropdown_field(
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
    options: Vec<(&'static str, &'static str)>,
    read: fn(&Preferences) -> &'static str,
    write: fn(&mut Preferences, &'static str),
) -> SettingField {
    let label = label.into();
    let description = description.into();
    SettingField {
        label: label.clone(),
        description: description.clone(),
        type_id: TypeId::of::<SharedString>(),
        render: Box::new(move |id_prefix, cx| {
            let cur = read(&SettingsStore::get(cx).prefs);
            let opts = options.clone();
            let id = SharedString::from(format!("{id_prefix}-dd"));

            h_flex()
                .gap_1()
                .child(
                    Button::new(SharedString::from(format!("{id}-val")))
                        .label(cur)
                        .on_click(move |_, _, cx| {
                            // Cycle to next option
                            let current = read(&SettingsStore::get(cx).prefs);
                            let next = cycle_option(&opts, current);
                            write_setting(move |p| write(p, next), cx);
                        }),
                )
                .into_any_element()
        }),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn write_setting(f: impl FnOnce(&mut Preferences), cx: &mut App) {
    SettingsStore::get_mut(cx).update_and_save(f);
    notify_panel(cx);
}

fn cycle_option(options: &[(&'static str, &'static str)], current: &'static str) -> &'static str {
    let pos = options.iter().position(|(_, v)| *v == current);
    let next = match pos {
        Some(i) if i + 1 < options.len() => i + 1,
        _ => 0,
    };
    options[next].1
}
