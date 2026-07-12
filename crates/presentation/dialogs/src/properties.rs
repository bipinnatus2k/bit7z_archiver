use bit7z_domain::archive::ArchiveEntry;
use bit7z_domain::repository::ArchiveProperties;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::description_list::{DescriptionItem, DescriptionList};
use gpui_component::scroll::ScrollableElement;
use gpui_component::v_flex;
use gpui_component::Sizable;
use humansize::{format_size, BINARY};
use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, CloseAction, DialogContent, DialogFooter, DialogHeader, DialogTitle,
    WindowDialogOptions,
};

pub struct PropertiesDialog;

impl PropertiesDialog {
    pub fn open_entries(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) {
        if entries.len() > 1 {
            open_window_dialog_async(
                cx,
                WindowDialogOptions {
                    title: format!("{} Entries Properties", entries.len()).into(),
                    width: px(480.),
                    height: Some(px(500.)),
                    min_width: None,
                    min_height: None,
                    kind: WindowKind::Dialog,
                    close_action: CloseAction::RemoveWindow,
                    window_decorations: Some(WindowDecorations::Client),
                    window_background: WindowBackgroundAppearance::Opaque,
                },
                move |_window, cx| cx.new(move |_| MultiEntryContent::new(entries)),
            );
        } else if let Some(entry) = entries.into_iter().next() {
            open_window_dialog_async(
                cx,
                WindowDialogOptions {
                    title: "Entry Properties".into(),
                    width: px(480.),
                    height: Some(px(500.)),
                    min_width: None,
                    min_height: None,
                    kind: WindowKind::Dialog,
                    close_action: CloseAction::RemoveWindow,
                    window_decorations: Some(WindowDecorations::Client),
                    window_background: WindowBackgroundAppearance::Opaque,
                },
                move |_window, cx| cx.new(move |_| EntryContent { entry }),
            );
        }
    }

    pub fn open_archive(
        path: String,
        props: ArchiveProperties,
        cx: &mut AsyncApp,
    ) {
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Archive Properties".into(),
                width: px(480.),
                height: Some(px(500.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| cx.new(move |_| ArchiveContent { path, props: Some(props) }),
        );
    }
}

// ---------------------------------------------------------------------------
// Archive properties
// ---------------------------------------------------------------------------

struct ArchiveContent {
    path: String,
    props: Option<ArchiveProperties>,
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

impl Render for ArchiveContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Archive Properties")),
            )
            .child(
                DialogContent::new().child(
                    match self.props.as_ref() {
                        Some(props) => {
                            let type_label = props_type_label(props);
                            let ratio = if props.total_size > 0 {
                                format!("{:.0}%", (1.0 - props.packed_size as f64 / props.total_size as f64) * 100.0)
                            } else {
                                "0%".to_string()
                            };
                            let general_items = vec![
                                DescriptionItem::new("Type").value(type_label),
                                DescriptionItem::new("Location").value(self.path.clone()),
                                DescriptionItem::new("Size").value(format!("{}({})", props.total_size, format_size(props.total_size, BINARY))),
                                DescriptionItem::new("Packed").value(format!("{}({})", props.packed_size, format_size(props.packed_size, BINARY))),
                                DescriptionItem::new("Ratio").value(ratio),
                                DescriptionItem::new("Files").value(format!("{}", props.files_count)),
                                DescriptionItem::new("Folders").value(format!("{}", props.folders_count)),
                                DescriptionItem::new("Headers size").value(format!("{}({})", props.headers_size, format_size(props.headers_size, BINARY))),
                                DescriptionItem::new("Volumes").value(format!("{}", props.volumes_count)),
                            ];
                            let mut advanced_items = vec![
                                DescriptionItem::new("Solid").value(bool_yn(props.is_solid)),
                                DescriptionItem::new("Encrypted").value(bool_yn(props.is_encrypted)),
                                DescriptionItem::new("Encrypted names").value(bool_yn(props.encrypted_names)),
                                DescriptionItem::new("Multi-volume").value(bool_yn(props.is_multi_volume)),
                                DescriptionItem::new("Has comment").value(bool_yn(props.has_comment)),
                                DescriptionItem::new("Recovery record").value(bool_yn(props.has_recovery_record)),
                                DescriptionItem::new("Locked").value(bool_yn(props.locked)),
                            ];
                            if let Some(sz) = props.dictionary_size {
                                advanced_items.push(DescriptionItem::new("Dictionary").value(format!("{}({})", sz, format_size(sz, BINARY))));
                            }

                            v_flex()
                                .gap_3()
                                .child(div().font_weight(FontWeight::MEDIUM).child("General"))
                                .child(DescriptionList::vertical().small().children(general_items))
                                .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Advanced"))
                                .child(DescriptionList::horizontal().bordered(false).small().children(advanced_items))
                                .into_any_element()
                        }
                        None => {
                            div().child("Loading properties...").into_any_element()
                        }
                    },
                ),
            )
            .child(
                DialogFooter::new().justify_end().child(
                    Button::new("close")
                        .label("Close")
                        .primary()
                        .on_click(|_, window, _| { window.remove_window(); }),
                ),
            )
    }
}

// ---------------------------------------------------------------------------
// Single entry properties
// ---------------------------------------------------------------------------

struct EntryContent {
    entry: ArchiveEntry,
}

impl Render for EntryContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry = &self.entry;
        let mut general_items = vec![
            DescriptionItem::new("Size").value(format!("{}({})", entry.size, format_size(entry.size, BINARY))),
            DescriptionItem::new("Packed").value(format!("{}({})", entry.compressed_size, format_size(entry.compressed_size, BINARY))),
            DescriptionItem::new("Ratio").value(format!("{:.0}%", entry.compression_ratio() * 100.0)),
        ];
        if let Some(crc) = entry.crc {
            general_items.push(DescriptionItem::new("CRC").value(format!("{:08X}", crc)));
        }

        let has_time = entry.modified.is_some() || entry.created.is_some() || entry.accessed.is_some();
        let has_platform = entry.host_os.is_some()
            || entry.attributes.is_some()
            || entry.posix_attrib.is_some()
            || entry.user.is_some()
            || entry.group.is_some();
        let has_tech = entry.compression_method.is_some() || entry.is_symlink || entry.comment.is_some();

        let mut content = v_flex()
            .gap_3()
            .child(div().font_weight(FontWeight::MEDIUM).child("General"))
            .child(DescriptionList::vertical().columns(1).children([
                DescriptionItem::new("Name").value(entry.name.clone()),
                DescriptionItem::new("Path").value(entry.path.clone()),
                DescriptionItem::new("Extension").value(entry.extension.as_deref().unwrap_or("-")),
            ]))
            .child(DescriptionList::vertical().columns(2).small().children(general_items));

        if has_time {
            let mut items = vec![];
            if let Some(ts) = entry.modified {
                items.push(DescriptionItem::new("Modified").value(ts.naive_local().to_string()));
            }
            if let Some(ts) = entry.created {
                items.push(DescriptionItem::new("Created").value(ts.to_string()));
            }
            if let Some(ts) = entry.accessed {
                items.push(DescriptionItem::new("Accessed").value(ts.to_string()));
            }
            content = content
                .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Time"))
                .child(DescriptionList::vertical().small().children(items));
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
                .child(DescriptionList::horizontal().bordered(false).small().columns(1).children(items));
        }

        if entry.is_encrypted {
            content = content
                .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Security"))
                .child(DescriptionList::horizontal().small().child(DescriptionItem::new("Encrypted").value("Yes")));
        }

        {
            let mut links = Vec::new();
            if let Some(ref hl) = entry.hardlink {
                links.push(DescriptionItem::new("Hardlink target").value(hl.clone()));
            }
            if !links.is_empty() {
                content = content
                    .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Links"))
                    .child(DescriptionList::horizontal().small().children(links));
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
                .child(DescriptionList::vertical().small().children(items));
        }

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Entry Properties")),
            )
            .child(
                DialogContent::new().child(
                    div().overflow_y_scrollbar().child(content),
                ),
            )
            .child(
                DialogFooter::new().justify_end().child(
                    Button::new("close")
                        .label("Close")
                        .primary()
                        .on_click(|_, window, _| { window.remove_window(); }),
                ),
            )
    }
}

// ---------------------------------------------------------------------------
// Multi-entry properties
// ---------------------------------------------------------------------------

struct MultiEntryContent {
    entries: Vec<ArchiveEntry>,
    show_entry_list: bool,
}

impl MultiEntryContent {
    fn new(entries: Vec<ArchiveEntry>) -> Self {
        Self { entries, show_entry_list: false }
    }
}

impl Render for MultiEntryContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_size: u64 = self.entries.iter().map(|e| e.size).sum();
        let total_packed: u64 = self.entries.iter().map(|e| e.compressed_size).sum();
        let entry_count = self.entries.len();

        let mut body = v_flex()
            .gap_3()
            .child(div().font_weight(FontWeight::MEDIUM).child("Totals"))
            .child(
                DescriptionList::horizontal()
                    .small()
                    .children([
                        DescriptionItem::new("Size").value(format_size(total_size, BINARY)),
                        DescriptionItem::new("Packed").value(format_size(total_packed, BINARY)),
                        DescriptionItem::new("Files").value(format!("{}", entry_count)),
                    ]),
            )
            .child(
                Button::new("toggle-list")
                    .label(if self.show_entry_list {
                        format!("\u{25bc} Entries ({})", entry_count)
                    } else {
                        format!("\u{25b6} Entries ({})", entry_count)
                    })
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_entry_list = !this.show_entry_list;
                        cx.notify();
                    })),
            );

        if self.show_entry_list {
            let mut list = v_flex()
                .gap_1()
                .child(
                    gpui_component::h_flex()
                        .gap_2()
                        .pb_1()
                        .child(div().w(px(200.)).text_xs().font_weight(FontWeight::BOLD).child("Name"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Size"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Packed")),
                );
            for e in &self.entries {
                list = list.child(
                    gpui_component::h_flex()
                        .gap_2()
                        .child(div().w(px(200.)).text_xs().overflow_hidden().child(e.path.clone()))
                        .child(div().w(px(80.)).text_xs().child(format_size(e.size, BINARY)))
                        .child(div().w(px(80.)).text_xs().child(format_size(e.compressed_size, BINARY))),
                );
            }
            body = body.child(list);
        }

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(format!("{} Entries Properties", entry_count))),
            )
            .child(
                DialogContent::new().child(
                    div().overflow_y_scrollbar().child(body),
                ),
            )
            .child(
                DialogFooter::new().justify_end().child(
                    Button::new("close")
                        .label("Close")
                        .primary()
                        .on_click(|_, window, _| { window.remove_window(); }),
                ),
            )
    }
}
