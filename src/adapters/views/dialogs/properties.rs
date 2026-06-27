use crate::domain::archive::ArchiveEntry;
use crate::domain::repository::ArchiveProperties;
use crate::theme::Theme;
use gpui::*;
use gpui_component::description_list::{DescriptionItem, DescriptionList};
use gpui_component::scroll::ScrollableElement;
use gpui_component::Sizable;
use gpui_component::WindowExt;

pub struct PropertiesDialog {
    mode: PropertiesMode,
    show_entry_list: bool,
}

pub enum PropertiesMode {
    Archive {
        path: String,
        properties: Option<ArchiveProperties>,
    },
    Entries(Vec<ArchiveEntry>),
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn bool_yn(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

fn props_type_label(props: &ArchiveProperties) -> String {
    let mut parts = Vec::new();
    if props.is_solid {
        parts.push("Solid");
    }
    if props.is_multi_volume {
        parts.push("Multi-volume");
    }
    if parts.is_empty() {
        "Archive".to_string()
    } else {
        parts.join(" ")
    }
}

impl PropertiesDialog {
    pub fn open_entries(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) {
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(200.), px(200.)),
                        size(px(480.), px(500.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_| PropertiesDialog {
                        mode: PropertiesMode::Entries(entries),
                        show_entry_list: false,
                    });
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
    }

    pub fn open_archive(path: String, props: ArchiveProperties, cx: &mut AsyncApp) {
        cx.spawn(async move |cx| {
            let bounds = cx.update(|app| {
                WindowBounds::centered(size(px(420.), px(380.)), app)
            });
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(bounds),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    kind: WindowKind::Dialog,
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_| PropertiesDialog {
                        mode: PropertiesMode::Archive {
                            path,
                            properties: Some(props),
                        },
                        show_entry_list: false,
                    });
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
    }
}

// --- private rendering helpers ---

fn render_archive_content(path: &str, props: Option<&ArchiveProperties>, theme: &Theme) -> impl IntoElement {
    let mut content = div().flex().flex_col().gap_1();

    content = match props {
        Some(props) => {
            let type_label = props_type_label(props);

            content
                .child(div().font_weight(FontWeight::MEDIUM).child("General"))
                .child(
                    DescriptionList::vertical()
                        // .bordered(false)
                        .small()
                        .children([
                            DescriptionItem::new("Type").value(type_label),
                            DescriptionItem::new("Location").value(path.to_string()),
                            DescriptionItem::new("Size").value(human_size(props.total_size)),
                            DescriptionItem::new("Packed").value(human_size(props.packed_size)),
                            DescriptionItem::new("Ratio")
                                .value(if props.total_size > 0 {
                                    format!("{:.0}%", (1.0 - props.packed_size as f64 / props.total_size as f64) * 100.0)
                                } else {
                                    "0%".to_string()
                                }),
                            DescriptionItem::new("Files").value(format!("{}", props.files_count)),
                            DescriptionItem::new("Folders").value(format!("{}", props.folders_count)),
                            DescriptionItem::new("Headers size").value(human_size(props.headers_size)),
                            DescriptionItem::new("Volumes").value(format!("{}", props.volumes_count)),
                        ]),
                )
                .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Advanced"))
                .child({
                    let mut items = vec![
                        DescriptionItem::new("Solid").value(bool_yn(props.is_solid)),
                        DescriptionItem::new("Encrypted").value(bool_yn(props.is_encrypted)),
                        DescriptionItem::new("Encrypted names").value(bool_yn(props.encrypted_names)),
                        DescriptionItem::new("Multi-volume").value(bool_yn(props.is_multi_volume)),
                        DescriptionItem::new("Has comment").value(bool_yn(props.has_comment)),
                        DescriptionItem::new("Recovery record").value(bool_yn(props.has_recovery_record)),
                        DescriptionItem::new("Locked").value(bool_yn(props.locked)),
                    ];
                    if let Some(sz) = props.dictionary_size {
                        items.push(DescriptionItem::new("Dictionary").value(human_size(sz)));
                    }
                    DescriptionList::horizontal().bordered(false).small().children(items)
                })
        }
        None => content.child(
            div().text_sm().text_color(theme.muted).child("Loading properties..."),
        ),
    };

    content
}

fn render_single_entry(entry: &ArchiveEntry) -> impl IntoElement {
    let mut general_items = vec![
        DescriptionItem::new("Size").value(human_size(entry.size)),
        DescriptionItem::new("Packed").value(human_size(entry.compressed_size)),
        DescriptionItem::new("Ratio")
            .value(format!("{:.0}%", entry.compression_ratio() * 100.0)),
    ];
    if let Some(crc) = entry.crc {
        general_items.push(DescriptionItem::new("CRC").value(format!("{:08X}", crc)));
    }

    let has_time =
        entry.modified.is_some() || entry.created.is_some() || entry.accessed.is_some();
    let has_platform = entry.host_os.is_some()
        || entry.attributes.is_some()
        || entry.posix_attrib.is_some()
        || entry.user.is_some()
        || entry.group.is_some();
    let has_tech =
        entry.compression_method.is_some() || entry.is_symlink || entry.comment.is_some();

    let mut content = div().flex().flex_col().gap_1()
        .child(div().font_weight(FontWeight::MEDIUM).child("General"))
        .child(
            DescriptionList::vertical()
                .children([
                    DescriptionItem::new("Name").value(entry.name.clone()),
                    DescriptionItem::new("Path").value(entry.path.clone()),
                    DescriptionItem::new("Extension").value(
                        entry.extension.as_deref().unwrap_or("-"),
                    ),
                ]),
        )
        .child(
            DescriptionList::vertical()
                .bordered(false)
                .small()
                .children(general_items),
        );

    if has_time {
        let mut items = vec![];
        if let Some(ts) = entry.modified {
            items.push(DescriptionItem::new("Modified").value(ts.to_string()));
        }
        if let Some(ts) = entry.created {
            items.push(DescriptionItem::new("Created").value(ts.to_string()));
        }
        if let Some(ts) = entry.accessed {
            items.push(DescriptionItem::new("Accessed").value(ts.to_string()));
        }
        content = content
            .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Time"))
            .child(
                DescriptionList::vertical()
                    .bordered(false)
                    .small()
                    .children(items),
            );
    }

    if has_platform {
        let mut items = vec![];
        if let Some(os) = entry.host_os {
            items.push(DescriptionItem::new("Host OS").value(format!("{}", os)));
        }
        if let Some(attr) = entry.attributes {
            items.push(DescriptionItem::new("Attributes").value(format!("{:08X}", attr)));
        }
        if let Some(posix) = entry.posix_attrib {
            items.push(DescriptionItem::new("POSIX").value(format!("{:o}", posix)));
        }
        if let Some(ref u) = entry.user {
            items.push(DescriptionItem::new("Owner").value(u.clone()));
        }
        if let Some(ref g) = entry.group {
            items.push(DescriptionItem::new("Group").value(g.clone()));
        }
        content = content
            .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Platform"))
            .child(
                DescriptionList::horizontal()
                    .bordered(false)
                    .small()
                    .children(items),
            );
    }

        if entry.is_encrypted {
            content = content
                .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Security"))
                .child(
                    DescriptionList::horizontal()
                        .bordered(false)
                        .small()
                        .child(DescriptionItem::new("Encrypted").value("Yes")),
                );
        }

        {
            let mut sec_items = Vec::new();
            if let Some(ref hl) = entry.hardlink {
                sec_items.push(DescriptionItem::new("Hardlink target").value(hl.clone()));
            }
            if !sec_items.is_empty() {
                content = content
                    .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Links"))
                    .child(
                        DescriptionList::horizontal()
                            .bordered(false)
                            .small()
                            .children(sec_items),
                    );
            }
        }

    if has_tech {
        let mut items = vec![];
        if let Some(ref method) = entry.compression_method {
            items.push(DescriptionItem::new("Method").value(method.clone()));
        }
        if entry.is_symlink {
            items.push(DescriptionItem::new("Symlink").value("Yes"));
        }
        if let Some(ref comment) = entry.comment {
            items.push(DescriptionItem::new("Comment").value(comment.clone()));
        }
        content = content
            .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Technical"))
            .child(
                DescriptionList::vertical()
                    .bordered(false)
                    .small()
                    .children(items),
            );
    }

    content
}

fn render_no_selection(theme: &Theme) -> impl IntoElement {
    div().text_sm().text_color(theme.muted).child("No entry selected")
}

impl Render for PropertiesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        let title = match &self.mode {
            PropertiesMode::Archive { .. } => "Archive Properties".to_string(),
            PropertiesMode::Entries(entries) if entries.len() > 1 => {
                format!("{} Entries Properties", entries.len())
            }
            _ => "Entry Properties".to_string(),
        };

        let content: gpui::AnyElement = match &mut self.mode {
            PropertiesMode::Archive { path, properties } => {
                render_archive_content(path, properties.as_ref(), &theme).into_any_element()
            }
            PropertiesMode::Entries(entries) if entries.len() > 1 => {
                let total_size: u64 = entries.iter().map(|e| e.size).sum();
                let total_packed: u64 = entries.iter().map(|e| e.compressed_size).sum();
                let entry_count = entries.len();
                let show_list = self.show_entry_list;

                let mut d = div().flex().flex_col().gap_1()
                    .child(div().font_weight(FontWeight::MEDIUM).child("Totals"))
                    .child(
                        DescriptionList::horizontal()
                            .bordered(false)
                            .small()
                            .children([
                                DescriptionItem::new("Size").value(human_size(total_size)),
                                DescriptionItem::new("Packed").value(human_size(total_packed)),
                                DescriptionItem::new("Files").value(format!("{}", entry_count)),
                            ]),
                    )
                    .child(
                        div().flex().flex_col().gap_1().child(
                            div()
                                .px_2()
                                .py_1()
                                .cursor_pointer()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .hover(|mut s| {
                                    s.background = Some(theme.hover.into());
                                    s
                                })
                                .child(if show_list { "\u{25BC}" } else { "\u{25B6}" })
                                .child(format!("Entries ({})", entry_count))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _e, _window, cx| {
                                        this.show_entry_list = !this.show_entry_list;
                                        cx.notify();
                                    }),
                                ),
                        ),
                    );

                if show_list {
                    let header = div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .pb_1()
                        .child(div().w(px(200.)).text_xs().font_weight(FontWeight::BOLD).child("Name"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Size"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Packed"));
                    d = d.child(header);
                    for e in entries.iter() {
                        d = d.child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .child(div().w(px(200.)).text_xs().overflow_hidden().child(e.path.clone()))
                                .child(div().w(px(80.)).text_xs().child(human_size(e.size)))
                                .child(div().w(px(80.)).text_xs().child(human_size(e.compressed_size))),
                        );
                    }
                }

                d.into_any_element()
            }
            PropertiesMode::Entries(entries) => {
                match entries.first() {
                    Some(entry) => render_single_entry(entry).into_any_element(),
                    None => render_no_selection(&theme).into_any_element(),
                }
            }
        };

        div().flex().flex_col().gap_2().p_4().size_full()
            .child(div().font_weight(FontWeight::BOLD).text_lg().child(title))
            .child(div().flex_1().overflow_y_scrollbar().child(content))
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(theme.primary)
                            .cursor_pointer()
                            .child("Close")
                            .on_mouse_down(MouseButton::Left, move |_e, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
            )
    }
}
