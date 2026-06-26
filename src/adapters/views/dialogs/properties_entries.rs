use crate::domain::archive::ArchiveEntry;
use crate::theme::Theme;
use crossbeam::channel::{unbounded, Sender};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::description_list::{DescriptionList, DescriptionItem};
use gpui_component::Sizable;
use std::sync::{Arc, Mutex};

pub struct PropertiesEntriesDialog {
    pub entries: Vec<ArchiveEntry>,
    pub show_entry_list: bool,
}

#[derive(Debug, Clone)]
pub enum PropertiesEntriesEvent {
    Close,
}

impl EventEmitter<PropertiesEntriesEvent> for PropertiesEntriesDialog {}

impl PropertiesEntriesDialog {
    pub fn new(entries: Vec<ArchiveEntry>) -> Self {
        Self {
            entries,
            show_entry_list: false,
        }
    }

    pub fn open(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) {
        let (tx, _rx) = unbounded::<PropertiesEntriesEvent>();
        let tx: Arc<Mutex<Option<Sender<PropertiesEntriesEvent>>>> = Arc::new(Mutex::new(Some(tx)));
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
                    let dialog = cx.new(|cx| PropertiesEntriesDialog::new(entries));
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        })
        .detach();
    }
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

fn build_single_props(entry: &ArchiveEntry) -> Div {
    let general_items = {
        let mut items = vec![
            DescriptionItem::new("Name").value(entry.name.clone()),
            DescriptionItem::new("Type")
                .value(if entry.is_directory { "Directory" } else { "File" }),
            DescriptionItem::new("Path").value(entry.path.clone()),
            DescriptionItem::new("Size").value(human_size(entry.size)),
            DescriptionItem::new("Packed").value(human_size(entry.compressed_size)),
            DescriptionItem::new("Ratio")
                .value(format!("{:.0}%", entry.compression_ratio() * 100.0)),
        ];
        if let Some(crc) = entry.crc {
            items.push(DescriptionItem::new("CRC").value(format!("{:08X}", crc)));
        }
        items
    };

    let has_time =
        entry.modified.is_some() || entry.created.is_some() || entry.accessed.is_some();
    let has_platform = entry.host_os.is_some()
        || entry.attributes.is_some()
        || entry.posix_attrib.is_some()
        || entry.user.is_some()
        || entry.group.is_some();
    let has_tech =
        entry.compression_method.is_some() || entry.is_symlink || entry.comment.is_some();

    div().flex().flex_col().gap_1()
        .child(div().font_weight(FontWeight::MEDIUM).child("General"))
        .child(
            DescriptionList::vertical()
                .bordered(false)
                .small()
                .children(general_items),
        )
        .when(has_time, |el| {
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
            el.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Time"))
                .child(
                    DescriptionList::vertical()
                        .bordered(false)
                        .small()
                        .children(items),
                )
        })
        .when(has_platform, |el| {
            let mut items = vec![];
            if let Some(os) = entry.host_os {
                items.push(DescriptionItem::new("Host OS").value(format!("{}", os)));
            }
            if let Some(attr) = entry.attributes {
                items.push(
                    DescriptionItem::new("Attributes").value(format!("{:08X}", attr)),
                );
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
            el.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Platform"))
                .child(
                    DescriptionList::vertical()
                        .bordered(false)
                        .small()
                        .children(items),
                )
        })
        .when(entry.is_encrypted, |el| {
            el.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Security"))
                .child(
                    DescriptionList::vertical()
                        .bordered(false)
                        .small()
                        .child(DescriptionItem::new("Encrypted").value("Yes")),
                )
        })
        .when(has_tech, |el| {
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
            el.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Technical"))
                .child(
                    DescriptionList::vertical()
                        .bordered(false)
                        .small()
                        .children(items),
                )
        })
}

impl Render for PropertiesEntriesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let multi = self.entries.len() > 1;

        let content: Div = if multi {
            let total_size: u64 = self.entries.iter().map(|e| e.size).sum();
            let total_packed: u64 = self.entries.iter().map(|e| e.compressed_size).sum();
            let entry_count = self.entries.len();
            let show_list = self.show_entry_list;

            let mut d = div().flex().flex_col().gap_1()
                .child(div().font_weight(FontWeight::MEDIUM).child("Totals"))
                .child(
                    DescriptionList::vertical()
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
                    .child(
                        div()
                            .w(px(200.))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("Name"),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("Size"),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("Packed"),
                    );
                d = d.child(header);
                for e in &self.entries {
                    d = d.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(200.))
                                    .text_xs()
                                    .overflow_hidden()
                                    .child(e.path.clone()),
                            )
                            .child(
                                div()
                                    .w(px(80.))
                                    .text_xs()
                                    .child(human_size(e.size)),
                            )
                            .child(
                                div()
                                    .w(px(80.))
                                    .text_xs()
                                    .child(human_size(e.compressed_size)),
                            ),
                    );
                }
            }
            d
        } else if let Some(entry) = self.entries.first() {
            build_single_props(entry)
        } else {
            div().child(
                div()
                    .text_sm()
                    .text_color(theme.muted)
                    .child("No entry selected"),
            )
        };

        div().flex().flex_col().gap_2().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child(
                if multi {
                    format!("{} Entries Properties", self.entries.len())
                } else {
                    "Entry Properties".to_string()
                },
            ))
            .child(content)
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _e, _window, cx| {
                                    cx.emit(PropertiesEntriesEvent::Close);
                                }),
                            ),
                    ),
            )
    }
}
