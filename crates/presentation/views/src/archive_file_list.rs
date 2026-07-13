use bit7z_pres_view_models::archive_state::{LevelEntry, ViewStatus};
use bit7z_pres_view_models::AppState;
use bit7z_pres_components::state_view::{empty_view, error_view, loading_view};
use bit7z_pres_components::ext_table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState};
use bit7z_app_checksum::ChecksumAlgorithm;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use serde::Deserialize;

macro_rules! emit_intents {
    ($builder:expr, $cx:expr, $( $action:ident => $intent:expr ),+ $(,)?) => {{
        let mut b = $builder;
        $(
            b = b.on_boxed_action(
                &$action,
                $cx.listener(|_, _, _, cx| {
                    cx.emit($intent);
                }),
            );
        )+
        b
    }};
}

macro_rules! bind_keys {
    ($cx:expr, $ns:expr, $( $key:expr => $action:expr ),+ $(,)?) => {
        $cx.bind_keys([
            $( KeyBinding::new($key, $action, Some($ns)), )+
        ])
    };
}

#[derive(Debug, Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = archive_file_list, no_json)]
pub enum FileListIntent {
    SelectionChanged(Vec<u32>),
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

actions!(archive_file_list,[
    SelectionChanged,
    SortByColumn,
    NavigateUp,
    OpenEntry,
    PreviewEntry,
    ExtractSelected,
    TestSelected,
    RenameEntry,
    DeleteSelected,
    ChecksumCRC32,
    ChecksumMD5,
    ChecksumSHA1,
    ChecksumSHA256,
    SelectAll,
    ClearSelection,
    Refresh,
    ShowProperties,
]);

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
        let fl_read = fl.read(cx);
        let has_selection = fl_read.state.has_selection();
        let single_selection = fl_read.state.selection_len() == 1;
        let ready = fl_read.state.is_ready();

        menu
        .when(has_selection,|m| {
            m
                .menu("Open",Box::new(OpenEntry))
                .when(single_selection,|m| {
                    m.menu("Preview",Box::new(PreviewEntry))
                })
                .menu("Extract",Box::new(ExtractSelected))
                .menu("Test",Box::new(TestSelected))
                .separator()
                .menu("Rename",Box::new(RenameEntry))
                .menu("Delete", Box::new(DeleteSelected))
                .separator()
                .item(PopupMenuItem::submenu("Checksum", PopupMenu::build(_window, cx, |menu, _window, _cx| {
                    menu.menu("CRC32",Box::new(ChecksumCRC32))
                        .menu("MD5",Box::new(ChecksumMD5))
                        .menu("SHA-1", Box::new(ChecksumSHA1))
                        .menu("SHA-256", Box::new(ChecksumSHA256))
                })))
        })
            .separator()
            .menu("Select All",Box::new(SelectAll))
            .when(has_selection, |m|{
                m.menu("Clear Selection", Box::new(ClearSelection))
            })
            .separator()
            .when(ready, |m|{
                m.menu("Refresh", Box::new(Refresh))
            })
            .menu("Properties", Box::new(ShowProperties))
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        let row = &self.entries[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "name" => row.display_name.clone(),
            "size" => row.size.to_string(),
            "packed" => row.compressed_size.to_string(),
            "ratio" => {
                let ratio = if row.size == 0 {
                    "0%".to_string()
                } else {
                    format!("{:.0}%", (1.0 - (row.compressed_size as f64 / row.size as f64)) * 100.)
                };
                ratio
            },
            "date" => {
                let date = row.modified
                                .map(|t| t.naive_local().to_string())
                                .unwrap_or_default();
                date
            },
            _ => "".to_string(),
        }
    }

    fn render_empty(&mut self, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        empty_view(cx,"")
    }
}

pub struct ArchiveFileList {
    state: AppState,
    table_state: Entity<TableState<FileListTableDelegate>>,
    focus_handle: FocusHandle,
}

impl ArchiveFileList {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, state: AppState) -> Self {
        bind_keys!(cx, "archive_file_list",
            "ctrl-a" => SelectAll,
            "ctrl-o" => OpenEntry,
            "ctrl-m" => RenameEntry,
            "ctrl-d" => DeleteSelected,
            "ctrl-p" => ShowProperties,
        );
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
                .multi_select(true)
                .col_selectable(true)
                .cell_selectable(false)
        });

        let state_for_selection = state.clone();
        cx.subscribe_in(&table_state, window, move |view: &mut Self, _table, event, _window, cx| {
            match event {
                TableEvent::SelectRow(_row_ix) => {
                    let entries = view.state.level_entries.get();
                    let indices: Vec<u32> = view.table_state.read(cx).selected_rows().iter()
                        .filter_map(|&row| entries.get(row))
                        .map(|e| e.original_index)
                        .collect();
                    state_for_selection.selection.update(|s| { *s = indices.iter().copied().collect(); });
                    state_for_selection.selection_anchor.set(None);
                    cx.emit(FileListIntent::SelectionChanged(indices));
                }
                TableEvent::DoubleClickedRow(_row_ix) => {
                    let entries = view.state.level_entries.get();
                    let indices: Vec<u32> = view.table_state.read(cx).selected_rows().iter()
                        .filter_map(|&row| entries.get(row))
                        .map(|e| e.original_index)
                        .collect();
                    state_for_selection.selection.update(|s| { *s = indices.iter().copied().collect(); });
                    state_for_selection.selection_anchor.set(None);
                    cx.emit(FileListIntent::SelectionChanged(indices));
                    cx.emit(FileListIntent::OpenEntry);
                }
                TableEvent::ClearSelection => {
                    state_for_selection.selection.update(|s| s.clear());
                    state_for_selection.selection_anchor.set(None);
                    cx.emit(FileListIntent::SelectionChanged(vec![]));
                }
                _ => {}
            }
        }).detach();

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window,cx);

        Self { state, table_state, focus_handle }
    }

    pub fn select_all_entries(&mut self, cx: &mut Context<Self>) {
        let entries = self.state.level_entries.get();
        let rows: std::collections::HashSet<usize> = (0..entries.len()).collect();
        self.state.selection.update(|s| {
            *s = entries.iter().map(|e| e.original_index).collect();
        });
        self.state.selection_anchor.set(None);
        self.table_state.update(cx, |state, cx| {
            state.set_selected_rows(rows, cx);
        });
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.state.selection.update(|s| s.clear());
        self.state.selection_anchor.set(None);
        self.table_state.update(cx, |state, cx| {
            state.set_selected_rows(std::collections::HashSet::new(), cx);
            state.clear_selection(cx);
        });
    }
}

impl Render for ArchiveFileList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.state.level_entries.get();
        let status = self.state.status.get();
        let current_path = self.state.current_path.get();

        self.table_state.update(cx, |state, _| {
            state.delegate_mut().entries = entries.clone();
        });

        let base = gpui_component::v_flex()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .key_context("archive_file_list")
            .track_focus(&self.focus_handle);

        let base = emit_intents!(base, cx,
            NavigateUp => FileListIntent::NavigateUp,
            OpenEntry => FileListIntent::OpenEntry,
            PreviewEntry => FileListIntent::PreviewEntry,
            ExtractSelected => FileListIntent::ExtractSelected,
            TestSelected => FileListIntent::TestSelected,
            RenameEntry => FileListIntent::RenameEntry(None),
            DeleteSelected => FileListIntent::DeleteSelected,
            SelectAll => FileListIntent::SelectAll,
            ClearSelection => FileListIntent::ClearSelection,
            Refresh => FileListIntent::Refresh,
            ShowProperties => FileListIntent::ShowProperties,
            ChecksumCRC32 => FileListIntent::Checksum(ChecksumAlgorithm::Crc32),
            ChecksumMD5 => FileListIntent::Checksum(ChecksumAlgorithm::Md5),
            ChecksumSHA1 => FileListIntent::Checksum(ChecksumAlgorithm::Sha1),
            ChecksumSHA256 => FileListIntent::Checksum(ChecksumAlgorithm::Sha256),
        );

        match &status {
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
                let path_str = current_path.trim_end_matches('/').to_string();
                let self_handle = cx.entity();

                let mut container = base;
                let h = self_handle.clone();
                container = container
                    .child(
                        Breadcrumb::new()
                            .bg(cx.theme().background)
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
                    div().flex_1().child(DataTable::new(&table_entity).scrollbar_visible(true, true))
                        .id("entry-table-area")
                )
            }
        }
    }
}
