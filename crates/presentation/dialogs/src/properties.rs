use bit7z_domain::archive::ArchiveEntry;
use bit7z_domain::repository::ArchiveProperties;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::description_list::{DescriptionItem, DescriptionList};
use gpui_component::list::{List, ListDelegate, ListItem, ListState};
use gpui_component::IndexPath;
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
            open_multi_entry_window(entries, cx)
        } else if let Some(entry) = entries.into_iter().next() {
            open_single_entry_window(entry, cx)
        }
    }

    pub fn open_archive(path: String, props: ArchiveProperties, cx: &mut AsyncApp) {
        open_archive_window(path, props, cx)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn opts(title: impl Into<SharedString>) -> WindowDialogOptions {
    WindowDialogOptions {
        title: title.into(),
        width: px(480.),
        height: Some(px(500.)),
        min_width: None,
        min_height: None,
        kind: WindowKind::Dialog,
        close_action: CloseAction::RemoveWindow,
        ..Default::default()
    }
}

fn bool_yn(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

fn kv_display(items: Vec<(&str, String)>, columns: usize) -> impl IntoElement {
    div()
        .flex_1()
        .overflow_y_scrollbar()
        .child(
            DescriptionList::vertical().columns(columns).small().children(
                items.into_iter().map(|(k, v)| DescriptionItem::new(k).value(v)),
            ),
        )
}

fn dialog_body(title: SharedString, content: impl IntoElement) -> impl IntoElement {
    v_flex()
        .size_full()
        .gap(px(12.))
        .child(DialogHeader::new().child(DialogTitle::new().child(title)))
        .child(DialogContent::new().child(content))
        .child(DialogFooter::new().justify_end().child(
            Button::new("close").label("Close").primary().on_click(|_, window, _| window.remove_window()),
        ))
}

// ---------------------------------------------------------------------------
// KV property display (used by both archive and single-entry)
// ---------------------------------------------------------------------------

struct PropertiesContent {
    title: SharedString,
    fields: Vec<(&'static str, String)>,
    columns: usize,
}

impl Render for PropertiesContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        dialog_body(self.title.clone(), kv_display(self.fields.clone(), self.columns))
    }
}

fn archive_fields(props: &ArchiveProperties, path: &str) -> Vec<(&'static str, String)> {
    let ratio = if props.total_size() > 0 {
        format!("{:.0}%", (1.0 - props.packed_size() as f64 / props.total_size() as f64) * 100.0)
    } else {
        "0%".into()
    };
    let mut kind = String::new();
    if props.is_solid() { kind.push_str("Solid "); }
    if props.is_multi_volume() { kind.push_str("Multi-volume "); }
    if kind.is_empty() { kind.push_str("Archive"); }

    vec![
        ("Type", kind),
        ("Location", path.into()),
        ("Size", format_size(props.total_size(), BINARY)),
        ("Packed", format_size(props.packed_size(), BINARY)),
        ("Ratio", ratio),
        ("Files", format!("{}", props.files_count())),
        ("Folders", format!("{}", props.folders_count())),
        ("Headers size", format_size(props.headers_size(), BINARY)),
        ("Volumes", format!("{}", props.volumes_count())),
        ("Solid", bool_yn(props.is_solid()).into()),
        ("Encrypted", bool_yn(props.is_encrypted()).into()),
        ("Encrypted names", bool_yn(props.encrypted_names()).into()),
        ("Multi-volume", bool_yn(props.is_multi_volume()).into()),
        ("Has comment", bool_yn(props.has_comment()).into()),
        ("Recovery record", bool_yn(props.has_recovery_record()).into()),
        ("Locked", bool_yn(props.locked()).into()),
        ("Dictionary size", props.dictionary_size().map(|s| format_size(s, BINARY)).unwrap_or_else(|| "-".into())),
    ]
}

fn dt_fmt(ts: Option<i64>) -> String {
    match ts {
        Some(t) if t >= 0 => {
            let days = t / 86400;
            let year = 1970 + (days / 365) as i32;
            let doy = (days % 365) as i32;
            let mut month = 1i32;
            let mut rem = doy;
            for dm in &[31i32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
                if rem < *dm { break; }
                rem -= *dm;
                month += 1;
            }
            format!("{:04}-{:02}-{:02}", year, month, rem + 1)
        }
        _ => "-".into(),
    }
}

fn entry_fields(entry: &ArchiveEntry) -> Vec<(&'static str, String)> {
    vec![
        ("Name", entry.name().to_string()),
        ("Path", entry.path().to_string()),
        ("Extension", entry.extension().unwrap_or("-").to_string()),
        ("Size", format_size(entry.size(), BINARY)),
        ("Packed", format_size(entry.compressed_size(), BINARY)),
        ("Ratio", format!("{:.0}%", entry.compression_ratio() * 100.0)),
        ("CRC", if entry.crc() != 0 { format!("{:08X}", entry.crc()) } else { "-".into() }),
        ("Modified", dt_fmt(entry.mtime())),
        ("Created", dt_fmt(entry.ctime())),
        ("Accessed", dt_fmt(entry.atime())),
        ("Host OS", format!("{}", entry.host_os())),
        ("Attributes", format!("{:08X}", entry.attributes())),
        ("POSIX", format!("{:o}", entry.posix_attrib())),
        ("Owner", entry.user().unwrap_or("-").to_string()),
        ("Group", entry.group().unwrap_or("-").to_string()),
        ("Encrypted", bool_yn(entry.is_encrypted()).into()),
        ("Symlink", bool_yn(entry.is_symlink()).into()),
        ("Method", entry.compression_method().unwrap_or("-").to_string()),
        ("Comment", entry.comment().unwrap_or("-").to_string()),
        ("Hardlink target", entry.hardlink().unwrap_or("-").to_string()),
    ]
}

fn open_archive_window(path: String, props: ArchiveProperties, cx: &mut AsyncApp) {
    let fields = archive_fields(&props, &path);
    open_window_dialog_async(cx, opts("Archive Properties"), move |_, cx| {
        cx.new(move |_| PropertiesContent { title: "Archive Properties".into(), fields, columns: 1 })
    });
}

fn open_single_entry_window(entry: ArchiveEntry, cx: &mut AsyncApp) {
    let fields = entry_fields(&entry);
    open_window_dialog_async(cx, opts("Entry Properties"), move |_, cx| {
        cx.new(move |_| PropertiesContent { title: "Entry Properties".into(), fields, columns: 2 })
    });
}

// ---------------------------------------------------------------------------
// Multi-entry properties
// ---------------------------------------------------------------------------

struct EntriesDelegate {
    entries: Vec<ArchiveEntry>,
}

impl ListDelegate for EntriesDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.entries.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut gpui::Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.entries.get(ix.row).map(|e| {
            ListItem::new(ix).child(
                gpui_component::h_flex()
                    .gap_2()
                    .w_full()
                    .child(div().w(px(200.)).text_xs().overflow_hidden().child(e.path().to_string()))
                    .child(div().w(px(80.)).text_xs().child(format_size(e.size(), BINARY)))
                    .child(div().w(px(80.)).text_xs().child(format_size(e.compressed_size(), BINARY))),
            )
        })
    }

    fn set_selected_index(
        &mut self,
        _ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut gpui::Context<ListState<Self>>,
    ) {
    }
}

fn open_multi_entry_window(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) {
    let total_size: u64 = entries.iter().map(|e| e.size()).sum();
    let total_packed: u64 = entries.iter().map(|e| e.compressed_size()).sum();
    let entry_count = entries.len();
    let title = SharedString::from(format!("{} Entries Properties", entry_count));

    open_window_dialog_async(cx, opts(title), move |window, cx| {
        let delegate = EntriesDelegate { entries };
        let list_state = cx.new(|cx| ListState::new(delegate, window, cx));
        cx.new(move |_| MultiEntryContent { list_state, total_size, total_packed, entry_count, show_entry_list: false })
    });
}

struct MultiEntryContent {
    list_state: Entity<ListState<EntriesDelegate>>,
    total_size: u64,
    total_packed: u64,
    entry_count: usize,
    show_entry_list: bool,
}

impl Render for MultiEntryContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let totals = DescriptionList::horizontal().small().children([
            DescriptionItem::new("Total size").value(format_size(self.total_size, BINARY)),
            DescriptionItem::new("Total packed").value(format_size(self.total_packed, BINARY)),
            DescriptionItem::new("Files").value(format!("{}", self.entry_count)),
        ]);

        let toggle = Button::new("toggle-list")
            .label(if self.show_entry_list {
                format!("\u{25bc} Entries ({})", self.entry_count)
            } else {
                format!("\u{25b6} Entries ({})", self.entry_count)
            })
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_entry_list = !this.show_entry_list;
                cx.notify();
            }));

        dialog_body(
            SharedString::from(format!("{} Entries Properties", self.entry_count)),
            v_flex()
                .gap_3()
                .child(totals)
                .child(toggle)
                .when(self.show_entry_list, |body| {
                    body.child(gpui_component::h_flex().gap_2().pb_1()
                        .child(div().w(px(200.)).text_xs().font_weight(FontWeight::BOLD).child("Name"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Size"))
                        .child(div().w(px(80.)).text_xs().font_weight(FontWeight::BOLD).child("Packed")))
                        .child(List::new(&self.list_state))
                }),
        )
    }
}
