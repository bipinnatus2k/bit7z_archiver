use bit7z_domain::archive::ArchiveFormat;
use bit7z_domain::preferences::DarkMode;
use gpui::{App, SharedString};
use gpui_component::setting::*;

use crate::backend::store::SettingsStore;
use crate::controls;
use crate::renderer;

pub fn general_page(cx: &mut App) -> SettingPage {
    SettingPage::new("General")
        .resettable(true)
        .default_open(true)
        .groups(vec![
            SettingGroup::new().title("Appearance").items(vec![
                SettingItem::new("Theme", controls::theme_picker_field(cx))
                    .description("Select a theme"),
                SettingItem::new("UI Font", controls::font_picker_field())
                    .description("Select the interface font"),
                renderer::string_dropdown(
                    "Theme Mode",
                    "Select light, dark, or system theme",
                    vec![
                        ("Light".into(), "light".into()),
                        ("Dark".into(), "dark".into()),
                        ("System".into(), "system".into()),
                    ],
                    |cx| {
                        let s = SettingsStore::get(cx);
                        match s.prefs.ui.night_mode {
                            DarkMode::Light => "light",
                            DarkMode::Dark => "dark",
                            DarkMode::System => "system",
                        }
                        .into()
                    },
                    |val, cx| {
                        let mode = match val.as_str() {
                            "light" => DarkMode::Light,
                            "dark" => DarkMode::Dark,
                            _ => DarkMode::System,
                        };
                        SettingsStore::get_mut(cx).update_and_save(|p| p.ui.night_mode = mode);
                    },
                ),
                renderer::bool_switch(
                    "Minimize to Tray",
                    "Minimize to system tray instead of taskbar",
                    |cx| SettingsStore::get(cx).prefs.ui.minimize_to_tray,
                    |val, cx| {
                        SettingsStore::get_mut(cx).update_and_save(|p| p.ui.minimize_to_tray = val);
                    },
                ),
                renderer::bool_switch(
                    "Confirm Delete",
                    "Show confirmation dialog before deleting files",
                    |cx| SettingsStore::get(cx).prefs.ui.confirm_delete,
                    |val, cx| {
                        SettingsStore::get_mut(cx).update_and_save(|p| p.ui.confirm_delete = val);
                    },
                ),
            ]),
            SettingGroup::new().title("Archive Defaults").items(vec![
                renderer::string_dropdown(
                    "Default Format",
                    "Default archive format for new archives",
                    vec![
                        ("7z".into(), "7z".into()),
                        ("ZIP".into(), "zip".into()),
                        ("TAR".into(), "tar".into()),
                        ("TAR.GZ".into(), "tar.gz".into()),
                        ("TAR.BZ2".into(), "tar.bz2".into()),
                        ("TAR.XZ".into(), "tar.xz".into()),
                        ("GZIP".into(), "gz".into()),
                        ("BZIP2".into(), "bz2".into()),
                        ("XZ".into(), "xz".into()),
                        ("WIM".into(), "wim".into()),
                    ],
                    |cx| {
                        let s = SettingsStore::get(cx);
                        format!("{:?}", s.prefs.archive.default_format)
                            .to_lowercase()
                            .into()
                    },
                    |val, cx| {
                        let format = match val.as_str() {
                            "zip" => ArchiveFormat::Zip,
                            "tar" => ArchiveFormat::Tar,
                            "tar.gz" => ArchiveFormat::TarGz,
                            "tar.bz2" => ArchiveFormat::TarBz2,
                            "tar.xz" => ArchiveFormat::TarXz,
                            "gz" => ArchiveFormat::GZip,
                            "bz2" => ArchiveFormat::BZip2,
                            "xz" => ArchiveFormat::Xz,
                            "wim" => ArchiveFormat::Wim,
                            _ => ArchiveFormat::SevenZip,
                        };
                        SettingsStore::get_mut(cx)
                            .update_and_save(|p| p.archive.default_format = format);
                    },
                ),
                renderer::number_input(
                    "Compression Level",
                    "Default compression level (0 = store, 9 = maximum)",
                    0.0,
                    9.0,
                    1.0,
                    |cx| {
                        SettingsStore::get(cx)
                            .prefs
                            .archive
                            .default_compression_level as f64
                    },
                    |val, cx| {
                        SettingsStore::get_mut(cx)
                            .update_and_save(|p| p.archive.default_compression_level = val as u8);
                    },
                ),
            ]),
        ])
}

pub fn all_pages(cx: &mut App) -> Vec<SettingPage> {
    vec![general_page(cx)]
}
