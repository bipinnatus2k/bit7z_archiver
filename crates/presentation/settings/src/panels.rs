use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::backend::store::SettingsStore;
use crate::field::{AnySettingField, SettingField, bool_field, f64_field, dropdown_field};
use bit7z_domain::preferences::{Preferences, ThemeMode};
use bit7z_domain::archive::ArchiveFormat;

// ── Section descriptor ───────────────────────────────────────────────────

struct Section {
    title: SharedString,
    items: Vec<Box<dyn AnySettingField>>,
}

// ── Entity ───────────────────────────────────────────────────────────────

pub struct SettingsPanel {
    sections: Vec<Section>,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            sections: vec![
                Section {
                    title: "Appearance".into(),
                    items: vec![
                        Box::new(bool_field(
                            "Minimize to Tray",
                            "Minimize to system tray instead of taskbar",
                            |p| p.ui.minimize_to_tray,
                            |p, v| p.ui.minimize_to_tray = v,
                        )),
                        Box::new(bool_field(
                            "Confirm Delete",
                            "Show confirmation dialog before deleting files",
                            |p| p.ui.confirm_delete,
                            |p, v| p.ui.confirm_delete = v,
                        )),
                        Box::new(dropdown_field(
                            "Theme Mode",
                            "",
                            vec![("Light", "light"), ("Dark", "dark"), ("System", "system")],
                            |p| match p.ui.theme {
                                ThemeMode::Light => "light",
                                ThemeMode::Dark => "dark",
                                ThemeMode::System => "system",
                            },
                            |p, v| {
                                p.ui.theme = match v {
                                    "light" => ThemeMode::Light,
                                    "dark" => ThemeMode::Dark,
                                    _ => ThemeMode::System,
                                };
                            },
                        )),
                    ],
                },
                Section {
                    title: "Archive Defaults".into(),
                    items: vec![
                        Box::new(dropdown_field(
                            "Default Format",
                            "",
                            vec![
                                ("7z", "7z"),
                                ("ZIP", "zip"),
                                ("TAR", "tar"),
                                ("TAR.GZ", "tar.gz"),
                                ("TAR.BZ2", "tar.bz2"),
                                ("TAR.XZ", "tar.xz"),
                            ],
                            |p| match p.archive.default_format {
                                ArchiveFormat::SevenZip => "7z",
                                ArchiveFormat::Zip => "zip",
                                ArchiveFormat::Tar => "tar",
                                ArchiveFormat::TarGz => "tar.gz",
                                ArchiveFormat::TarBz2 => "tar.bz2",
                                ArchiveFormat::TarXz => "tar.xz",
                                _ => "7z",
                            },
                            |p, v| {
                                p.archive.default_format = match v {
                                    "zip" => ArchiveFormat::Zip,
                                    "tar" => ArchiveFormat::Tar,
                                    "tar.gz" => ArchiveFormat::TarGz,
                                    "tar.bz2" => ArchiveFormat::TarBz2,
                                    "tar.xz" => ArchiveFormat::TarXz,
                                    _ => ArchiveFormat::SevenZip,
                                };
                            },
                        )),
                        Box::new(f64_field(
                            "Compression Level",
                            "0 = store, 9 = maximum",
                            |p| p.archive.default_compression_level as f64,
                            |p, v| p.archive.default_compression_level = v as u8,
                            0.0, 9.0,
                        )),
                    ],
                },
            ],
        }
    }
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fg = cx.theme().foreground;
        let primary = cx.theme().primary;
        let muted = cx.theme().muted;

        let mut children: Vec<AnyElement> = Vec::new();

        children.push(
            div()
                .text_color(fg)
                .text_lg()
                .child("Settings")
                .into_any_element(),
        );

        for section in &self.sections {
            let mut items: Vec<AnyElement> = Vec::new();
            for (idx, item) in section.items.iter().enumerate() {
                let id_prefix = format!("sec{idx}");
                let label = item.label();
                let desc = item.description();
                let control = item.render_control(&id_prefix, cx);

                let mut row = h_flex()
                    .id(SharedString::from(format!("item-{id_prefix}")))
                    .gap_2()
                    .items_center()
                    .child(div().text_color(fg).child(label))
                    .child(control);

                if !desc.is_empty() {
                    row = row.child(div().text_color(muted).child(desc));
                }

                items.push(row.into_any_element());
            }

            children.push(
                v_flex()
                    .gap_2()
                    .child(div().text_color(primary).text_sm().child(section.title.clone()))
                    .children(items)
                    .into_any_element(),
            );
        }

        v_flex().p_4().gap_4().children(children)
    }
}
