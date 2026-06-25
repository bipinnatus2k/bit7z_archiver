use crate::adapters::events::ChecksumAlgorithm;
use crate::adapters::view_models::archive_state::{LevelEntry, ViewStatus};
use crate::adapters::views::components::state_view::{empty_view, error_view, loading_view};
use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState};
use humansize::{format_size, BINARY};

#[derive(Debug, Clone, PartialEq)]
pub enum FileListIntent {
    RowClicked(usize),
    SortByColumn(u32, bool),
    NavigateUp,
    OpenEntry,
    PreviewEntry,
    ExtractSelected,
    TestSelected,
    RenameEntry(Option<u32>),
    DeleteSelected,
    Checksum(ChecksumAlgorithm),
    SelectAll,
    ClearSelection,
    Refresh,
    ShowProperties,
}

impl EventEmitter<FileListIntent> for ArchiveFileList {}

struct FileListTableDelegate {
    entries: Vec<LevelEntry>,
    columns: Vec<Column>,
    file_list: gpui::WeakEntity<ArchiveFileList>,
}

impl TableDelegate for FileListTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.entries.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, _window: &mut Window, _cx: &mut Context<TableState<Self>>) {
        if sort == ColumnSort::Default { return; }
        let ascending = sort == ColumnSort::Ascending;
        let key = self.columns[col_ix].key.clone();
        self.entries.sort_by(|a, b| {
            let ord = match key.as_str() {
                "name" => a.display_name.cmp(&b.display_name),
                "size" => a.size.cmp(&b.size),
                "packed" => a.compressed_size.cmp(&b.compressed_size),
                "ratio" => {
                    let ra = if a.size == 0 { 0.0 } else { 1.0 - a.compressed_size as f64 / a.size as f64 };
                    let rb = if b.size == 0 { 0.0 } else { 1.0 - b.compressed_size as f64 / b.size as f64 };
                    ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
                }
                "date" => a.modified.cmp(&b.modified),
                _ => std::cmp::Ordering::Equal,
            };
            if ascending { ord } else { ord.reverse() }
        });
        if let Some(fl) = self.file_list.upgrade() {
            fl.update(_cx, |_, cx| cx.emit(FileListIntent::SortByColumn(col_ix as u32, ascending)));
        }
    }

    fn context_menu(
        &mut self,
        _row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        let fl = match self.file_list.upgrade() {
            Some(f) => f,
            None => return menu,
        };
        let has_selection = fl.read(cx).selection.len() > 0;
        let single_selection = fl.read(cx).selection.len() == 1;
        let ready = fl.read(cx).is_ready;

        let mut m = menu;
        if has_selection {
            let h1 = fl.clone();
            m = m.item(PopupMenuItem::new("Open").on_click(move |_, _, cx| {
                h1.update(cx, |_, cx| cx.emit(FileListIntent::OpenEntry));
            }));
            if single_selection {
                let h2 = fl.clone();
                m = m.item(PopupMenuItem::new("Preview").on_click(move |_, _, cx| {
                    h2.update(cx, |_, cx| cx.emit(FileListIntent::PreviewEntry));
                }));
            }
            let h3 = fl.clone();
            m = m.item(PopupMenuItem::new("Extract...").on_click(move |_, _, cx| {
                h3.update(cx, |_, cx| cx.emit(FileListIntent::ExtractSelected));
            }));
            let h_test = fl.clone();
            m = m.item(PopupMenuItem::new("Test...").on_click(move |_, _, cx| {
                h_test.update(cx, |_, cx| cx.emit(FileListIntent::TestSelected));
            }));
            m = m.separator();
            if single_selection {
                let h4 = fl.clone();
                m = m.item(PopupMenuItem::new("Rename").on_click(move |_, _, cx| {
                    h4.update(cx, |_, cx| cx.emit(FileListIntent::RenameEntry(None)));
                }));
            }
            let h5 = fl.clone();
            m = m.item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                h5.update(cx, |_, cx| cx.emit(FileListIntent::DeleteSelected));
            }));
            m = m.separator();
            let h_crc32 = fl.clone();
            m = m.item(PopupMenuItem::new("CRC32").on_click(move |_, _, cx| {
                h_crc32.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Crc32)));
            }));
            let h_md5 = fl.clone();
            m = m.item(PopupMenuItem::new("MD5").on_click(move |_, _, cx| {
                h_md5.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Md5)));
            }));
            let h_sha1 = fl.clone();
            m = m.item(PopupMenuItem::new("SHA1").on_click(move |_, _, cx| {
                h_sha1.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Sha1)));
            }));
            let h_sha256 = fl.clone();
            m = m.item(PopupMenuItem::new("SHA256").on_click(move |_, _, cx| {
                h_sha256.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Sha256)));
            }));
        }
        m = m.separator();
        let h6 = fl.clone();
        m = m.item(PopupMenuItem::new("Select All").on_click(move |_, _, cx| {
            h6.update(cx, |_, cx| cx.emit(FileListIntent::SelectAll));
        }));
        if has_selection {
            let h7 = fl.clone();
            m = m.item(PopupMenuItem::new("Clear Selection").on_click(move |_, _, cx| {
                h7.update(cx, |_, cx| cx.emit(FileListIntent::ClearSelection));
            }));
        }
        m = m.separator();
        if ready {
            let h8 = fl.clone();
            m = m.item(PopupMenuItem::new("Refresh").on_click(move |_, _, cx| {
                h8.update(cx, |_, cx| cx.emit(FileListIntent::Refresh));
            }));
        }
        if single_selection {
            let h9 = fl.clone();
            m = m.item(PopupMenuItem::new("Properties").on_click(move |_, _, cx| {
                h9.update(cx, |_, cx| cx.emit(FileListIntent::ShowProperties));
            }));
        }
        m
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let entry = match self.entries.get(row_ix) {
            Some(e) => e,
            None => return div(),
        };
        match self.columns[col_ix].key.as_str() {
            "name" => {
                let text = if entry.is_directory {
                    format!("\u{1f4c1} {}", entry.display_name)
                } else {
                    format!("\u{1f4c4} {}", entry.display_name)
                };
                div().px_2().child(text)
            }
            "size" => div().px_2().child(format_size(entry.size, BINARY)),
            "packed" => div().px_2().child(format_size(entry.compressed_size, BINARY)),
            "ratio" => {
                let ratio = if entry.size == 0 {
                    "0%".to_string()
                } else {
                    format!("{:.0}%", (1.0 - (entry.compressed_size as f64 / entry.size as f64)) * 100.)
                };
                div().px_2().child(ratio)
            }
            "date" => {
                let date = entry.modified
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                div().px_2().child(date)
            }
            _ => div(),
        }
    }
}

pub struct ArchiveFileList {
    entries: Vec<LevelEntry>,
    selection: std::collections::HashSet<u32>,
    status: ViewStatus,
    current_path: String,
    is_ready: bool,
    table_state: Entity<TableState<FileListTableDelegate>>,
}

impl ArchiveFileList {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let columns = vec![
            Column::new("name", "Name").width(300.).sortable(),
            Column::new("size", "Size").width(80.).sortable(),
            Column::new("packed", "Packed").width(80.).sortable(),
            Column::new("ratio", "Ratio").width(80.).sortable(),
            Column::new("date", "Date").width(140.).sortable(),
        ];
        let delegate = FileListTableDelegate {
            entries: vec![],
            columns,
            file_list: cx.entity().downgrade(),
        };
        let table_state = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .row_selectable(true)
                .col_selectable(true)
                .cell_selectable(false)
        });

        cx.subscribe_in(&table_state, window, |_view, _table, event, _window, cx| {
            match event {
                TableEvent::SelectRow(row_ix) => {
                    cx.emit(FileListIntent::RowClicked(*row_ix));
                }
                TableEvent::DoubleClickedRow(row_ix) => {
                    cx.emit(FileListIntent::RowClicked(*row_ix));
                    cx.emit(FileListIntent::OpenEntry);
                }
                TableEvent::ClearSelection => {
                    cx.emit(FileListIntent::ClearSelection);
                }
                _ => {}
            }
        }).detach();

        Self { entries: vec![], selection: std::collections::HashSet::new(), status: ViewStatus::Empty, current_path: String::new(), is_ready: false, table_state }
    }

    pub fn set_state(&mut self, entries: Vec<LevelEntry>, selection: std::collections::HashSet<u32>, status: ViewStatus, current_path: String, cx: &mut Context<Self>) {
        self.entries = entries.clone();
        self.selection = selection;
        self.status = status;
        self.current_path = current_path;
        self.is_ready = self.status == ViewStatus::Ready;
        self.table_state.update(cx, |state, _| {
            state.delegate_mut().entries = entries;
        });
    }
}

impl Render for ArchiveFileList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let base = gpui_component::v_flex()
            .flex_1()
            .border_b_1()
            .border_color(theme.border);

        match &self.status {
            ViewStatus::Empty => {
                base.child(empty_view(cx, "Open an archive to browse its contents"))
            }
            ViewStatus::Loading => {
                base.child(loading_view(cx))
            }
            ViewStatus::Error(msg) => {
                base.child(error_view(cx, msg))
            }
            ViewStatus::Ready => {
                let path_str = self.current_path.trim_end_matches('/').to_string();
                let self_handle = cx.entity();

                let mut container = base;
                let h = self_handle.clone();
                container = container
                    .child(
                        Breadcrumb::new()
                            .bg(theme.surface)
                            .child(
                                BreadcrumbItem::new(" \u{2190} ")
                                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                        h.update(cx, |_, cx| cx.emit(FileListIntent::NavigateUp));
                                    })
                            )
                            .child(
                                BreadcrumbItem::new(path_str)
                            )
                    );

                let table_entity = self.table_state.clone();
                container.child(
                    div().flex_1().child(DataTable::new(&table_entity))
                        .id("entry-table-area")
                )
            }
        }
    }
}
