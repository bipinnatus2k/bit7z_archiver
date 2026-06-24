use crate::adapters::events::ChecksumAlgorithm;
use crate::adapters::view_models::archive_state::{LevelEntry, ViewStatus};
use crate::adapters::views::components::state_view::{empty_view, error_view, loading_view};
use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::table::{Column as TableColumn, ColumnSort, DataTable, TableDelegate, TableState};
use humansize::{format_size, BINARY};

#[derive(Debug, Clone, PartialEq)]
pub enum FileListIntent {
    RowClicked(usize, gpui::Modifiers),
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
    file_list: gpui::WeakEntity<ArchiveFileList>,
}

fn read_fl<F, R>(fl: &gpui::WeakEntity<ArchiveFileList>, cx: &App, f: F) -> Option<R>
where F: FnOnce(&ArchiveFileList) -> R {
    fl.upgrade().map(|fl| f(&fl.read(cx)))
}

impl TableDelegate for FileListTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize { 5 }

    fn rows_count(&self, cx: &App) -> usize {
        read_fl(&self.file_list, cx, |fl| fl.entries.len()).unwrap_or(0)
    }

    fn column(&self, col_ix: usize, _cx: &App) -> TableColumn {
        match col_ix {
            0 => TableColumn::new("name", "Name").width(px(300.)).sortable(),
            1 => TableColumn::new("size", "Size").width(px(80.)).sortable(),
            2 => TableColumn::new("packed", "Packed").width(px(80.)).sortable(),
            3 => TableColumn::new("ratio", "Ratio").width(px(80.)).sortable(),
            4 => TableColumn::new("date", "Date").width(px(140.)).sortable(),
            _ => unreachable!(),
        }
    }

    fn perform_sort(&mut self, col_ix: usize, sort: ColumnSort, _window: &mut Window, cx: &mut Context<TableState<Self>>) {
        if sort == ColumnSort::Default { return; }
        if let Some(fl) = self.file_list.upgrade() {
            fl.update(cx, |_, cx| cx.emit(FileListIntent::SortByColumn(col_ix as u32, sort == ColumnSort::Ascending)));
        }
    }

    fn render_tr(&mut self, row_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> Stateful<Div> {
        let selected = read_fl(&self.file_list, cx, |fl| {
            fl.entries.get(row_ix).map(|e| fl.selection.contains(&e.original_index))
        }).flatten().unwrap_or(false);
        let theme = cx.global::<Theme>();

        let fl_left = self.file_list.clone();
        let fl_dbl = self.file_list.clone();
        let fl_right = self.file_list.clone();
        div().id(("row", row_ix))
            .cursor_pointer()
            .when(selected, |d| d.bg(theme.selection))
            .when(!selected && row_ix % 2 == 0, |d| d.bg(theme.surface))
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                if let Some(fl) = fl_left.upgrade() {
                    fl.update(cx, |_, cx| cx.emit(FileListIntent::RowClicked(row_ix, event.modifiers.clone())));
                    if event.click_count >= 2 {
                        fl.update(cx, |_, cx| cx.emit(FileListIntent::OpenEntry));
                    }
                }
            })
            .on_mouse_down(MouseButton::Right, move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                if let Some(fl) = fl_right.upgrade() {
                    fl.update(cx, |_, cx| cx.emit(FileListIntent::RowClicked(row_ix, Default::default())));
                }
            })
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let entry = read_fl(&self.file_list, cx, |fl| fl.entries.get(row_ix).cloned()).flatten();

        match col_ix {
            0 => {
                if let Some(e) = entry {
                    let text = if e.is_directory {
                        "\u{1f4c1} ".to_string() + &e.display_name
                    } else {
                        "\u{1f4c4} ".to_string() + &e.display_name
                    };
                    div().px_2().child(text)
                } else { div() }
            }
            1 => div().px_2().child(entry.map(|e| format_size(e.size, BINARY)).unwrap_or_default()),
            2 => div().px_2().child(entry.map(|e| format_size(e.compressed_size, BINARY)).unwrap_or_default()),
            3 => {
                let ratio = entry.map(|e| if e.size == 0 { "0%".to_string() } else {
                    format!("{:.0}%", (1.0 - (e.compressed_size as f64 / e.size as f64)) * 100.)
                }).unwrap_or_default();
                div().px_2().child(ratio)
            }
            4 => {
                let date = entry.and_then(|e| e.modified)
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
        let delegate = FileListTableDelegate {
            file_list: cx.entity().downgrade(),
        };
        let table_state = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .row_selectable(false)
                .cell_selectable(false)
        });
        Self { entries: vec![], selection: std::collections::HashSet::new(), status: ViewStatus::Empty, current_path: String::new(), is_ready: false, table_state }
    }

    pub fn set_state(&mut self, entries: Vec<LevelEntry>, selection: std::collections::HashSet<u32>, status: ViewStatus, current_path: String) {
        self.entries = entries;
        self.selection = selection;
        self.status = status;
        self.current_path = current_path;
        self.is_ready = self.status == ViewStatus::Ready;
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
                let is_root = self.current_path.is_empty();
                let path_str = self.current_path.trim_end_matches('/').to_string();
                let self_handle = cx.entity();

                let mut container = base;
                if !is_root {
                    let h = self_handle.clone();
                    container = container
                        .child(
                            div().flex().flex_row().gap_1().px_2().py_1()
                                .bg(theme.surface)
                                .child(
                                    div().cursor_pointer().child(" \u{2190} ")
                                        .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                            h.update(cx, |_, cx| cx.emit(FileListIntent::NavigateUp));
                                        })
                                )
                                .child(format!(" \u{1f4c2} {}", path_str))
                        );
                }

                let table_entity = self.table_state.clone();
                let has_selection = !self.selection.is_empty();
                let single_selection = self.selection.len() == 1;
                let ready = self.is_ready;
                let h = self_handle.clone();

                container.child(
                    div().flex_1().child(DataTable::new(&table_entity).stripe(false).bordered(false))
                        .id("entry-table-area")
                        .context_menu(move |menu, window, cx| {
                            let mut m = menu;
                            if has_selection {
                                let h1 = h.clone();
                                m = m.item(PopupMenuItem::new("Open").on_click(move |_, _, cx| {
                                    h1.update(cx, |_, cx| cx.emit(FileListIntent::OpenEntry));
                                }));
                                if single_selection {
                                    let h2 = h.clone();
                                    m = m.item(PopupMenuItem::new("Preview").on_click(move |_, _, cx| {
                                        h2.update(cx, |_, cx| cx.emit(FileListIntent::PreviewEntry));
                                    }));
                                }
                                let h3 = h.clone();
                                m = m.item(PopupMenuItem::new("Extract...").on_click(move |_, _, cx| {
                                    h3.update(cx, |_, cx| cx.emit(FileListIntent::ExtractSelected));
                                }));
                                let h_test = h.clone();
                                m = m.item(PopupMenuItem::new("Test...").on_click(move |_, _, cx| {
                                    h_test.update(cx, |_, cx| cx.emit(FileListIntent::TestSelected));
                                }));
                                m = m.separator();
                                if single_selection {
                                    let h4 = h.clone();
                                    m = m.item(PopupMenuItem::new("Rename").on_click(move |_, _, cx| {
                                        h4.update(cx, |_, cx| cx.emit(FileListIntent::RenameEntry(None)));
                                    }));
                                }
                                let h5 = h.clone();
                                m = m.item(PopupMenuItem::new("Delete").on_click(move |_, _, cx| {
                                    h5.update(cx, |_, cx| cx.emit(FileListIntent::DeleteSelected));
                                }));
                                m = m.separator();
                                let h_ck = h.clone();
                                m = m.submenu("Checksum", window, cx, move |menu, _, _| {
                                    let h_crc32 = h_ck.clone();
                                    let h_md5 = h_ck.clone();
                                    let h_sha1 = h_ck.clone();
                                    let h_sha256 = h_ck.clone();
                                    menu.item(PopupMenuItem::new("CRC32").on_click(move |_, _, cx| {
                                        h_crc32.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Crc32)));
                                    }))
                                    .item(PopupMenuItem::new("MD5").on_click(move |_, _, cx| {
                                        h_md5.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Md5)));
                                    }))
                                    .item(PopupMenuItem::new("SHA1").on_click(move |_, _, cx| {
                                        h_sha1.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Sha1)));
                                    }))
                                    .item(PopupMenuItem::new("SHA256").on_click(move |_, _, cx| {
                                        h_sha256.update(cx, |_, cx| cx.emit(FileListIntent::Checksum(ChecksumAlgorithm::Sha256)));
                                    }))
                                });
                            }
                            m = m.separator();
                            let h6 = h.clone();
                            m = m.item(PopupMenuItem::new("Select All").on_click(move |_, _, cx| {
                                h6.update(cx, |_, cx| cx.emit(FileListIntent::SelectAll));
                            }));
                            if has_selection {
                                let h7 = h.clone();
                                m = m.item(PopupMenuItem::new("Clear Selection").on_click(move |_, _, cx| {
                                    h7.update(cx, |_, cx| cx.emit(FileListIntent::ClearSelection));
                                }));
                            }
                            m = m.separator();
                            if ready {
                                let h8 = h.clone();
                                m = m.item(PopupMenuItem::new("Refresh").on_click(move |_, _, cx| {
                                    h8.update(cx, |_, cx| cx.emit(FileListIntent::Refresh));
                                }));
                            }
                            if single_selection {
                                let h9 = h.clone();
                                m = m.item(PopupMenuItem::new("Properties").on_click(move |_, _, cx| {
                                    h9.update(cx, |_, cx| cx.emit(FileListIntent::ShowProperties));
                                }));
                            }
                            m
                        })
                )
            }
        }
    }
}
