use crate::domain::archive::ArchiveEntry;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

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

fn prop_row(label: &str, value: String, theme: &Theme) -> Div {
    div().flex().flex_row().gap_4().py_1p5()
        .child(div().w(px(100.)).text_sm().text_color(theme.muted).child(label.to_string()))
        .child(div().text_sm().child(value))
}

fn build_single_props(entry: &ArchiveEntry, theme: &Theme) -> Div {
    let mut root = div().flex().flex_col().gap_1();

    // General
    root = root.child(div().font_weight(FontWeight::MEDIUM).child("General"));
    root = root.child(prop_row("Name:", entry.name.clone(), theme));
    root = root.child(prop_row("Type:", if entry.is_directory { "Directory" } else { "File" }.to_string(), theme));
    root = root.child(prop_row("Path:", entry.path.clone(), theme));
    root = root.child(prop_row("Size:", human_size(entry.size), theme));
    root = root.child(prop_row("Packed:", human_size(entry.compressed_size), theme));
    root = root.child(prop_row("Ratio:", format!("{:.0}%", entry.compression_ratio() * 100.0), theme));
    if let Some(crc) = entry.crc {
        root = root.child(prop_row("CRC:", format!("{:08X}", crc), theme));
    }

    // Time
    let has_time = entry.modified.is_some() || entry.created.is_some() || entry.accessed.is_some();
    if has_time {
        root = root.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Time"));
        if let Some(ts) = entry.modified {
            root = root.child(prop_row("Modified:", ts.to_string(), theme));
        }
        if let Some(ts) = entry.created {
            root = root.child(prop_row("Created:", ts.to_string(), theme));
        }
        if let Some(ts) = entry.accessed {
            root = root.child(prop_row("Accessed:", ts.to_string(), theme));
        }
    }

    // Platform
    let has_platform = entry.host_os.is_some() || entry.attributes.is_some()
        || entry.posix_attrib.is_some() || entry.user.is_some() || entry.group.is_some();
    if has_platform {
        root = root.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Platform"));
        if let Some(os) = entry.host_os {
            root = root.child(prop_row("Host OS:", format!("{}", os), theme));
        }
        if let Some(attr) = entry.attributes {
            root = root.child(prop_row("Attributes:", format!("{:08X}", attr), theme));
        }
        if let Some(posix) = entry.posix_attrib {
            root = root.child(prop_row("POSIX:", format!("{:o}", posix), theme));
        }
        if let Some(ref u) = entry.user {
            root = root.child(prop_row("Owner:", u.clone(), theme));
        }
        if let Some(ref g) = entry.group {
            root = root.child(prop_row("Group:", g.clone(), theme));
        }
    }

    // Security
    if entry.is_encrypted {
        root = root.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Security"));
        root = root.child(prop_row("Encrypted:", "Yes".to_string(), theme));
    }

    // Technical
    let has_tech = entry.compression_method.is_some() || entry.is_symlink || entry.comment.is_some();
    if has_tech {
        root = root.child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Technical"));
        if let Some(ref method) = entry.compression_method {
            root = root.child(prop_row("Method:", method.clone(), theme));
        }
        if entry.is_symlink {
            root = root.child(prop_row("Symlink:", "Yes".to_string(), theme));
        }
        if let Some(ref comment) = entry.comment {
            root = root.child(prop_row("Comment:", comment.clone(), theme));
        }
    }

    root
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
                .child(prop_row("Size:", human_size(total_size), &theme))
                .child(prop_row("Packed:", human_size(total_packed), &theme))
                .child(prop_row("Files:", format!("{}", entry_count), &theme))
                .child(
                    div().flex().flex_col().gap_1()
                        .child(
                            div().px_2().py_1().cursor_pointer().flex().flex_row().gap_1()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child(if show_list { "\u{25BC}" } else { "\u{25B6}" })
                                .child(format!("Entries ({})", entry_count))
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    this.show_entry_list = !this.show_entry_list;
                                    cx.notify();
                                }))
                        )
                );

            if show_list {
                let header = div().flex().flex_row().gap_2().pb_1()
                    .child(div().w(px(200.)).text_xs().font_weight(FontWeight::BOLD).child("Name"))
                    .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Size"))
                    .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Packed"));
                d = d.child(header);
                for e in &self.entries {
                    d = d.child(
                        div().flex().flex_row().gap_2()
                            .child(div().w(px(200.)).text_xs().overflow_hidden().child(e.path.clone()))
                            .child(div().w(px(80.)).text_xs().child(human_size(e.size)))
                            .child(div().w(px(80.)).text_xs().child(human_size(e.compressed_size)))
                    );
                }
            }
            d
        } else if let Some(entry) = self.entries.first() {
            build_single_props(entry, &theme)
        } else {
            div().child(div().text_sm().text_color(theme.muted).child("No entry selected"))
        };

        div().flex().flex_col().gap_2().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child(
                if multi { format!("{} Entries Properties", self.entries.len()) }
                else { "Entry Properties".to_string() }
            ))
            .child(content)
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(
                        div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                cx.emit(PropertiesEntriesEvent::Close);
                            }))
                    )
            )
    }
}
