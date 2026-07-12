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
            move |_window, cx| cx.new(move |_| ArchiveContent { path, props }),
        );
    }
}

// ---------------------------------------------------------------------------
// Archive properties
// ---------------------------------------------------------------------------

struct ArchiveContent {
    path: String,
    props: ArchiveProperties,
}

fn archive_fields<'a>(props: &'a ArchiveProperties, path: &'a str) -> Vec<(&'a str, String)> {
    let ratio = if props.total_size > 0 {
        format!("{:.0}%", (1.0 - props.packed_size as f64 / props.total_size as f64) * 100.0)
    } else {
        "0%".into()
    };
    let mut kind = String::new();
    if props.is_solid { kind.push_str("Solid "); }
    if props.is_multi_volume { kind.push_str("Multi-volume "); }
    if kind.is_empty() { kind.push_str("Archive"); }

    vec![
        ("Type", kind),
        ("Location", path.into()),
        ("Size", format_size(props.total_size, BINARY)),
        ("Packed", format_size(props.packed_size, BINARY)),
        ("Ratio", ratio),
        ("Files", format!("{}", props.files_count)),
        ("Folders", format!("{}", props.folders_count)),
        ("Headers size", format_size(props.headers_size, BINARY)),
        ("Volumes", format!("{}", props.volumes_count)),
        ("Solid", bool_yn(props.is_solid)),
        ("Encrypted", bool_yn(props.is_encrypted)),
        ("Encrypted names", bool_yn(props.encrypted_names)),
        ("Multi-volume", bool_yn(props.is_multi_volume)),
        ("Has comment", bool_yn(props.has_comment)),
        ("Recovery record", bool_yn(props.has_recovery_record)),
        ("Locked", bool_yn(props.locked)),
        ("Dictionary size", props.dictionary_size.map(|s| format_size(s, BINARY)).unwrap_or_else(|| "-".into())),
    ]
}

fn bool_yn(v: bool) -> String {
    if v { "Yes".into() } else { "No".into() }
}

impl Render for ArchiveContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Archive Properties")))
            .child(
                DialogContent::new().child(
                    div().overflow_y_scrollbar().child(
                        DescriptionList::vertical().small().children(
                            archive_fields(&self.props, &self.path)
                                .into_iter()
                                .map(|(k, v)| DescriptionItem::new(k).value(v)),
                        ),
                    ),
                ),
            )
            .child(DialogFooter::new().justify_end().child(
                Button::new("close").label("Close").primary().on_click(|_, window, _| window.remove_window()),
            ))
    }
}

// ---------------------------------------------------------------------------
// Single entry properties
// ---------------------------------------------------------------------------

struct EntryContent {
    entry: ArchiveEntry,
}

fn entry_fields(entry: &ArchiveEntry) -> Vec<(&str, String)> {
    vec![
        ("Name", entry.name.clone()),
        ("Path", entry.path.clone()),
        ("Extension", entry.extension.clone().unwrap_or_else(|| "-".into())),
        ("Size", format_size(entry.size, BINARY)),
        ("Packed", format_size(entry.compressed_size, BINARY)),
        ("Ratio", format!("{:.0}%", entry.compression_ratio() * 100.0)),
        ("CRC", entry.crc.map(|c| format!("{:08X}", c)).unwrap_or_else(|| "-".into())),
        ("Modified", entry.modified.map(|t| t.naive_local().to_string()).unwrap_or_else(|| "-".into())),
        ("Created", entry.created.map(|t| t.to_string()).unwrap_or_else(|| "-".into())),
        ("Accessed", entry.accessed.map(|t| t.to_string()).unwrap_or_else(|| "-".into())),
        ("Host OS", entry.host_os.map(|o| format!("{}", o)).unwrap_or_else(|| "-".into())),
        ("Attributes", entry.attributes.map(|a| format!("{:08X}", a)).unwrap_or_else(|| "-".into())),
        ("POSIX", entry.posix_attrib.map(|p| format!("{:o}", p)).unwrap_or_else(|| "-".into())),
        ("Owner", entry.user.clone().unwrap_or_else(|| "-".into())),
        ("Group", entry.group.clone().unwrap_or_else(|| "-".into())),
        ("Encrypted", if entry.is_encrypted { "Yes".into() } else { "No".into() }),
        ("Symlink", if entry.is_symlink { "Yes".into() } else { "No".into() }),
        ("Method", entry.compression_method.clone().unwrap_or_else(|| "-".into())),
        ("Comment", entry.comment.clone().unwrap_or_else(|| "-".into())),
        ("Hardlink target", entry.hardlink.clone().unwrap_or_else(|| "-".into())),
    ]
}

impl Render for EntryContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let items = entry_fields(&self.entry);

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Entry Properties")))
            .child(
                DialogContent::new().child(
                    div().overflow_y_scrollbar().child(
                        DescriptionList::vertical().columns(2).small().children(
                            items.into_iter().map(|(k, v)| DescriptionItem::new(k).value(v)),
                        ),
                    ),
                ),
            )
            .child(DialogFooter::new().justify_end().child(
                Button::new("close").label("Close").primary().on_click(|_, window, _| window.remove_window()),
            ))
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

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child(format!("{} Entries Properties", entry_count))))
            .child(
                DialogContent::new().child(
                    div().overflow_y_scrollbar().child(
                        v_flex()
                            .gap_3()
                            .child(
                                DescriptionList::horizontal().small().children([
                                    DescriptionItem::new("Total size").value(format_size(total_size, BINARY)),
                                    DescriptionItem::new("Total packed").value(format_size(total_packed, BINARY)),
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
                            )
                            .when(self.show_entry_list, |body| {
                                let mut list = v_flex().gap_1()
                                    .child(
                                        gpui_component::h_flex().gap_2().pb_1()
                                            .child(div().w(px(200.)).text_xs().font_weight(FontWeight::BOLD).child("Name"))
                                            .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Size"))
                                            .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Packed")),
                                    );
                                for e in &self.entries {
                                    list = list.child(
                                        gpui_component::h_flex().gap_2()
                                            .child(div().w(px(200.)).text_xs().overflow_hidden().child(e.path.clone()))
                                            .child(div().w(px(80.)).text_xs().child(format_size(e.size, BINARY)))
                                            .child(div().w(px(80.)).text_xs().child(format_size(e.compressed_size, BINARY))),
                                    );
                                }
                                body.child(list)
                            }),
                    ),
                ),
            )
            .child(DialogFooter::new().justify_end().child(
                Button::new("close").label("Close").primary().on_click(|_, window, _| window.remove_window()),
            ))
    }
}
