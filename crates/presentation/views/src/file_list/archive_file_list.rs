use std::collections::HashSet;
use std::rc::Rc;
use bit7z_pres_components::state_view::{empty_view, error_view, loading_view};
use bit7z_pres_components::ext_table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState};
use bit7z_app_checksum::ChecksumAlgorithm;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{v_flex, ActiveTheme, Icon, IconName, Sizable, StyledExt};
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::native_menu::NativeMenu;
use serde::Deserialize;
use crate::file_list::{LevelEntry, ViewStatus};
use crate::file_list::archive_fm_state::FileListState;

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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum FileListEvent {
    SelectionChanged(Vec<u32>),
    // SortByColumn(u32, bool),
    // NavigateUp,
    OpenEntry,
    // PreviewEntry,
    // ExtractSelected,
    // TestSelected,
    // RenameEntry(Option<u32>),
    // DeleteSelected,
    // Checksum(ChecksumAlgorithm),
    // SelectAll,
    // ClearSelection,
    // Refresh,
    // ShowProperties,
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

#[derive(IntoElement)]
pub struct ArchiveFileList {
    state: Entity<FileListState>,
    style: StyleRefinement,
    // size: gpui_component::Size,

    // selection: HashSet<u32>,
    // status: ViewStatus,
    // current_path: String,
    // is_ready: bool,
    // table_state: Entity<TableState<FileListTableDelegate>>,
    // focus_handle: FocusHandle,
    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Styled for ArchiveFileList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ArchiveFileList {

    pub fn init(cx: &mut App) {
        bind_keys!(cx, "archive_file_list",
            "ctrl-a" => SelectAll,
            // "ctrl-o" => OpenEntry,
            // "ctrl-m" => RenameEntry,
            "ctrl-d" => DeleteSelected,
            // "ctrl-p" => ShowProperties,
        );
    }


    pub fn new(state: &Entity<FileListState>) -> Self {
        // let columns = vec![
        //     Column::new("name", "Name").width(300.).sortable(),
        //     Column::new("size", "Size").width(80.).sortable(),
        //     Column::new("packed", "Packed").width(80.).sortable(),
        //     Column::new("ratio", "Ratio").width(80.).sortable(),
        //     Column::new("date", "Date").width(140.).sortable(),
        // ];
        // let delegate = FileListTableDelegate {
        //     entries: vec![],
        //     columns,
        //     file_list: cx.entity().downgrade(),
        // };
        // let table_state = cx.new(|cx| {
        //     TableState::new(delegate, window, cx)
        //         .row_selectable(true)
        //         .multi_select(true)
        //         .col_selectable(true)
        //         .cell_selectable(false)
        // });
        //
        // cx.subscribe_in(&table_state, window, |view, _table, event, _window, cx| {
        //     match event {
        //         TableEvent::SelectRow(_row_ix) => {
        //             let indices: Vec<u32> = view.table_state.read(cx).selected_rows().iter()
        //                 .filter_map(|&row| view.entries.get(row))
        //                 .map(|e| e.original_index)
        //                 .collect();
        //             view.selection = indices.iter().copied().collect();
        //             cx.emit(FileListEvent::SelectionChanged(indices));
        //         }
        //         TableEvent::DoubleClickedRow(_row_ix) => {
        //             let indices: Vec<u32> = view.table_state.read(cx).selected_rows().iter()
        //                 .filter_map(|&row| view.entries.get(row))
        //                 .map(|e| e.original_index)
        //                 .collect();
        //             view.selection = indices.iter().copied().collect();
        //             cx.emit(FileListEvent::SelectionChanged(indices));
        //             cx.emit(FileListEvent::OpenEntry);
        //         }
        //         TableEvent::ClearSelection => {
        //             view.selection.clear();
        //             cx.emit(FileListEvent::SelectionChanged(vec![]));
        //         }
        //         // TableEvent::SelectColumn(_) => {}
        //         // TableEvent::SelectCell(_, _) => {}
        //         // TableEvent::DoubleClickedCell(_, _) => {}
        //         TableEvent::ColumnWidthsChanged(_w) => {
        //             //TODO: save widths config
        //         }
        //         TableEvent::MoveColumn(_, _) => {
        //             //TODO: save sort config
        //         }
        //         TableEvent::RightClickedRow(_) => {}
        //         // TableEvent::RightClickedCell(_, _) => {}
        //         _ => {}
        //     }
        // }).detach();
        //


        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            context_menu_builder: None,
            // entries: vec![],
            // selection: HashSet::new(),
            // status: ViewStatus::Empty,
            // current_path: String::new(),
            // is_ready: false,
            // table_state,
        }
    }


    // pub fn view(window: &mut Window, cx: &mut App) -> Entity<ArchiveFileList> {
    //     cx.new(|cx| { Self::new(window,cx) })
    // }

    // pub fn select_all_entries(&mut self, cx: &mut Context<Self>) {
    //     let rows: HashSet<usize> = (0..self.entries.len()).collect();
    //     self.selection = self.entries.iter().map(|e| e.original_index).collect();
    //     self.table_state.update(cx, |state, cx| {
    //         state.set_selected_rows(rows, cx);
    //     });
    // }
    //
    // pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
    //     self.selection.clear();
    //     self.table_state.update(cx, |state, cx| {
    //         state.set_selected_rows(HashSet::new(), cx);
    //         state.clear_selection(cx);
    //     });
    // }
    //
    // pub fn set_state(&mut self, entries: Vec<LevelEntry>, status: ViewStatus, current_path: String, cx: &mut Context<Self>) {
    //     self.entries = entries.clone();
    //     self.status = status;
    //     self.current_path = current_path;
    //     self.is_ready = self.status == ViewStatus::Ready;
    //     self.table_state.update(cx, |state, _| {
    //         state.delegate_mut().entries = entries;
    //     });
    // }
}

impl RenderOnce for ArchiveFileList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            // .border_b_1()
            // .border_color(cx.theme().border)
            .key_context("archive_file_list")
            .track_focus(&self.state.read(cx).focus_handle.clone())
            .child(self.state.clone())

    }
}

// impl Render for ArchiveFileList {
//     fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//         let base = v_flex()
//             .w_full()
//             .border_b_1()
//             .border_color(cx.theme().border)
//             .key_context("archive_file_list")
//             .track_focus(&self.focus_handle);
//
//         // let base = emit_intents!(base, cx,
//         //     NavigateUp => FileListEvent::NavigateUp,
//         //     OpenEntry => FileListEvent::OpenEntry,
//         //     PreviewEntry => FileListEvent::PreviewEntry,
//         //     ExtractSelected => FileListEvent::ExtractSelected,
//         //     TestSelected => FileListEvent::TestSelected,
//         //     RenameEntry => FileListEvent::RenameEntry(None),
//         //     DeleteSelected => FileListEvent::DeleteSelected,
//         //     SelectAll => FileListEvent::SelectAll,
//         //     ClearSelection => FileListEvent::ClearSelection,
//         //     Refresh => FileListEvent::Refresh,
//         //     ShowProperties => FileListEvent::ShowProperties,
//         //     ChecksumCRC32 => FileListEvent::Checksum(ChecksumAlgorithm::Crc32),
//         //     ChecksumMD5 => FileListEvent::Checksum(ChecksumAlgorithm::Md5),
//         //     ChecksumSHA1 => FileListEvent::Checksum(ChecksumAlgorithm::Sha1),
//         //     ChecksumSHA256 => FileListEvent::Checksum(ChecksumAlgorithm::Sha256),
//         // );
//
//         match &self.status {
//             ViewStatus::Empty => {
//                 base.child(empty_view(cx, "Open an archive to browse its contents"))
//             }
//             ViewStatus::Loading => {
//                 base.child(loading_view(cx))
//             }
//             ViewStatus::Error(msg) => {
//                 base.child(error_view(cx, msg))
//             }
//             ViewStatus::Ready => {
//                 // let path_str = self.current_path.trim_end_matches('/').to_string();
//                 // let paths = path_str.split('/').collect::<Vec<&str>>();
//                 let self_handle = cx.entity();
//
//                 let mut container = base;
//                 let h = self_handle.clone();
//                 // container = container
//                 //     .h_flex()
//                 //     .w_full()
//                 //     .child(
//                 //         Button::new("nav_up")
//                 //             .ghost()
//                 //             .icon(Icon::new(IconName::FolderUp))
//                 //             .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
//                 //                 h.update(cx, |_, cx| cx.emit(FileListEvent::NavigateUp));
//                 //             })
//                 //     );
//
//                 // container = container
//                 //     .child(
//                 //         Breadcrumb::new()
//                 //             .bg(cx.theme().background)
//                 //             .children(
//                 //                 paths.iter().map(|p| BreadcrumbItem::new(*p)).collect::<Vec<BreadcrumbItem>>()
//                 //             )
//                 // );
//
//                 let table_entity = self.table_state.clone();
//                 container.child(
//                     div().flex_1().child(DataTable::new(&table_entity).scrollbar_visible(true, true))
//                         .id("entry-table-area")
//                 )
//             }
//         }
//     }
// }
