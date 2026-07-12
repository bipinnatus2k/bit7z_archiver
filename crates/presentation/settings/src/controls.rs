use gpui::*;
use gpui_component::setting::*;
use gpui_component::{ActiveTheme, Theme};

use crate::backend::store::SettingsStore;

/// Compression level number field (0-9).
pub fn compression_level_field() -> SettingField<f64> {
    let options = NumberFieldOptions {
        min: 0.0,
        max: 9.0,
        step: 1.0,
    };
    SettingField::number_input(
        options,
        |cx: &App| {
            SettingsStore::get(cx).prefs.archive.default_compression_level as f64
        },
        |val: f64, cx: &mut App| {
            SettingsStore::get_mut(cx).update_and_save(|p| {
                p.archive.default_compression_level = val as u8;
            });
        },
    )
    .default_value(5.0)
}

/// Theme picker field — populates options from ThemeRegistry.
pub fn theme_picker_field(cx: &App) -> SettingField<SharedString> {
    let registry = gpui_component::theme::ThemeRegistry::global(cx);
    let options: Vec<(SharedString, SharedString)> = registry
        .themes()
        .keys()
        .map(|name| (name.clone(), name.clone()))
        .collect();

    SettingField::dropdown(
        options,
        |cx: &App| cx.theme().theme_name().clone(),
        |val: SharedString, cx: &mut App| {
            let mode = gpui_component::theme::ThemeRegistry::global(cx)
                .themes()
                .get(&val)
                .map(|config| config.mode);
            if let Some(mode) = mode {
                Theme::change(mode, None, cx);
                let prefs_mode = match mode {
                    gpui_component::theme::ThemeMode::Light => {
                        bit7z_domain::preferences::ThemeMode::Light
                    }
                    gpui_component::theme::ThemeMode::Dark => {
                        bit7z_domain::preferences::ThemeMode::Dark
                    }
                };
                SettingsStore::get_mut(cx).update_and_save(|p| {
                    p.ui.theme = prefs_mode;
                });
            }
        },
    )
}

/// Font picker field — simplified with common font list.
pub fn font_picker_field() -> SettingField<SharedString> {
    let common_fonts = vec![
        "System UI",
        "Segoe UI",
        "Arial",
        "Consolas",
        "Cascadia Code",
        "Fira Code",
        "JetBrains Mono",
        "Source Code Pro",
    ];

    let options: Vec<(SharedString, SharedString)> = common_fonts
        .iter()
        .map(|&name| (name.into(), name.into()))
        .collect();

    SettingField::dropdown(
        options,
        |cx: &App| cx.theme().font_family.clone(),
        |val: SharedString, cx: &mut App| {
            Theme::global_mut(cx).font_family = val;
        },
    )
}
