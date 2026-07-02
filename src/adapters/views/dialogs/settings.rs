use crate::domain::preferences::*;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};
use std::sync::{Arc, Mutex};
use gpui_component::{h_flex, v_flex, TitleBar};

type SharedSender<T> = Arc<Mutex<Option<Sender<T>>>>;

pub struct SettingsDialog {
    pub prefs: Preferences,
}

#[derive(Debug, Clone)]
pub enum SettingsDialogEvent {
    Saved(Preferences),
    Canceled,
}

impl EventEmitter<SettingsDialogEvent> for SettingsDialog {}

impl SettingsDialog {
    pub fn new(cx: &mut Context<Self>) -> Entity<Self> {
        let prefs = cx.global::<crate::gui::PreferencesGlobal>().0.clone();
        cx.new(|_cx| Self { prefs })
    }

    /// Open as independent window. Returns receiver for dialog events.
    pub fn open(cx: &mut AsyncApp) -> Receiver<SettingsDialogEvent> {
        let (tx, rx) = unbounded::<SettingsDialogEvent>();
        let event_tx: SharedSender<SettingsDialogEvent> = Arc::new(Mutex::new(Some(tx)));
        let et = event_tx.clone();
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    titlebar: TitlebarOptions {
                        title: Some(SharedString::from("Settings")),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }.into(),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(150.), px(150.)),
                        size(px(480.), px(500.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    // kind: WindowKind::Dialog,
                    ..Default::default()
                },
                move |window, cx| {
                    let prefs = cx.global::<crate::gui::PreferencesGlobal>().0.clone();
                    let dialog = cx.new(|_cx| SettingsDialog { prefs });
                    let et = et.clone();
                    cx.subscribe::<SettingsDialog, SettingsDialogEvent>(&dialog, move |_, evt: &SettingsDialogEvent, _| {
                        if let Ok(guard) = et.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.send(evt.clone());
                            }
                        }
                    }).detach();
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
        rx
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let prefs = self.prefs.clone();

        v_flex()
            .size_full()
            .child(
                Settings::new("app-settings")
                    .sidebar_width(px(180.))
                    .pages(vec![
                        // General
                        SettingPage::new("General")
                            .default_open(true)
                            .group(
                                SettingGroup::new()
                                    .title("Application")
                                    .items(vec![
                                        SettingItem::new(
                                            "Minimize to tray",
                                            SettingField::switch(
                                                move |_| prefs.ui.minimize_to_tray,
                                                |_val, _| {
                                                    
                                                },
                                            )
                                        )
                                        .description("Keep the application running in the system tray when minimized."),
                                        SettingItem::new(
                                            "Confirm before delete",
                                            SettingField::switch(
                                                move |_| prefs.ui.confirm_delete,
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Show a confirmation dialog before deleting archive entries."),
                                    ])
                            ),
                        // Archive
                        SettingPage::new("Archive")
                            .group(
                                SettingGroup::new()
                                    .title("Defaults")
                                    .items(vec![
                                        SettingItem::new(
                                            "Default format",
                                            SettingField::dropdown(
                                                writable_formats(),
                                                move |_| SharedString::from(prefs.archive.default_format.display_name()),
                                                |_val, _| {},
                                            )
                                        )
                                        .description("The default archive format when creating new archives."),
                                        SettingItem::new(
                                            "Compression level",
                                            SettingField::dropdown(
                                                vec![
                                                    ("0 - None".into(), "0".into()),
                                                    ("1 - Fastest".into(), "1".into()),
                                                    ("2 - Fast".into(), "2".into()),
                                                    ("3 - Normal".into(), "3".into()),
                                                    ("4 - Maximum".into(), "4".into()),
                                                    ("5 - Ultra".into(), "5".into()),
                                                ],
                                                move |_| SharedString::from(format!("{} - {}", prefs.archive.default_compression_level, compression_level_name(prefs.archive.default_compression_level))),
                                                |_val, _| {},
                                            )
                                        )
                                        .description("The default compression level for new archives."),
                                        SettingItem::new(
                                            "Encrypt filenames",
                                            SettingField::switch(
                                                move |_| prefs.archive.default_encrypt_filenames,
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Encrypt file names in the archive by default."),
                                    ]),
                            )
                            .group(
                                SettingGroup::new()
                                    .title("Recent Files")
                                    .item(
                                        SettingItem::render(move |_, _, _| {
                                            let recent = prefs.archive.recent_files.clone();
                                            if recent.is_empty() {
                                                return div().text_sm().text_color(gpui::black()).child("No recent files").into_any_element();
                                            }
                                            div().flex().flex_col().gap_1()
                                                .children(recent.into_iter().take(5).map(|f| {
                                                    div().text_sm().truncate().child(f).into_any_element()
                                                }).collect::<Vec<_>>())
                                                .into_any_element()
                                        }),
                                    ),
                            ),
                        // Preview
                        SettingPage::new("Preview")
                            .group(
                                SettingGroup::new()
                                    .title("Preview Options")
                                    .items(vec![
                                        SettingItem::new(
                                            "Auto-preview files",
                                            SettingField::switch(
                                                move |_| prefs.preview.auto_preview,
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Automatically preview files when selected."),
                                        SettingItem::new(
                                            "Text preview max size (KB)",
                                            SettingField::number_input(
                                                gpui_component::setting::NumberFieldOptions {
                                                    min: 64.0,
                                                    max: 10240.0,
                                                    step: 64.0,
                                                    ..Default::default()
                                                },
                                                move |_| (prefs.preview.text_max_bytes / 1024) as f64,
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Maximum file size in KB for text preview."),
                                        SettingItem::new(
                                            "Hex dump bytes",
                                            SettingField::number_input(
                                                gpui_component::setting::NumberFieldOptions {
                                                    min: 256.0,
                                                    max: 65536.0,
                                                    step: 256.0,
                                                    ..Default::default()
                                                },
                                                move |_| prefs.preview.hex_dump_bytes as f64,
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Number of bytes to show in hex dump view."),
                                    ]),
                            ),
                        // Appearance
                        SettingPage::new("Appearance")
                            .group(
                                SettingGroup::new()
                                    .title("Theme")
                                    .item(
                                        SettingItem::new(
                                            "Color scheme",
                                            SettingField::dropdown(
                                                vec![
                                                    ("Light".into(), "light".into()),
                                                    ("Dark".into(), "dark".into()),
                                                    ("System".into(), "system".into()),
                                                ],
                                                move |_| SharedString::from(match prefs.ui.theme {
                                                    ThemeMode::Light => "Light",
                                                    ThemeMode::Dark => "Dark",
                                                    ThemeMode::System => "System",
                                                }),
                                                |_val, _| {},
                                            )
                                        )
                                        .description("Choose your preferred color scheme."),
                                    ),
                            ),
                    ]),
            )
            // Action buttons
            .child(
                h_flex().justify_end().gap_2().pt_2()
                    .child(
                        Button::new("cancel")
                            .label("Cancel")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsDialogEvent::Canceled)))
                    )
                    .child(
                        Button::new("save")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.emit(SettingsDialogEvent::Saved(this.prefs.clone()));
                            }))
                    )
            )
    }
}

fn writable_formats() -> Vec<(SharedString, SharedString)> {
    vec![
        ("7Z".into(), "7z".into()),
        ("ZIP".into(), "zip".into()),
        ("TAR".into(), "tar".into()),
        ("TAR.GZ".into(), "tar.gz".into()),
        ("TAR.BZ2".into(), "tar.bz2".into()),
        ("TAR.XZ".into(), "tar.xz".into()),
    ]
}

fn compression_level_name(level: u8) -> &'static str {
    match level {
        0 => "None",
        1 => "Fastest",
        2 => "Fast",
        3 => "Normal",
        4 => "Maximum",
        5 => "Ultra",
        _ => "Normal",
    }
}
