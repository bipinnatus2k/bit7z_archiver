use crate::file_list::archive_fm_state::FileListState;
use crate::file_list::LevelEntry;
use bit7z_pres_components::ext_table::{Column, ColumnSort, TableDelegate, TableState};
use bit7z_pres_components::state_view::{empty_view, loading_view};
use gpui::{prelude::FluentBuilder, App, Context, IntoElement, WeakEntity, Window};
use gpui_component::menu::PopupMenu;

pub(crate) struct FileListTableDelegate {
    pub(crate) entries: Vec<LevelEntry>,
    pub(crate) columns: Vec<Column>,
    pub(crate) state: WeakEntity<FileListState>,

}

impl TableDelegate for FileListTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        let n = self.entries.len();
        log::info!("DataTable rows_count: {}", n);
        n
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
        // if let Some(state) = self.state.upgrade() {
        //     state.update(cx, |state, _| {
        //         state.sort_column = col_ix as u32;
        //         state.sort_ascending = ascending;
        //     });
        // }
    }

    fn context_menu(
        &mut self,
        _row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        // let has_selection = self.state.upgrade().map_or(false, |s| s.read(cx).has_selection());
        // let single_selection = self.state.upgrade().map_or(false, |s| s.read(cx).selection.len() == 1);
        // let is_ready = self.state.upgrade().map_or(false, |s| s.read(cx).is_ready());
        //
        menu
        //     .when(has_selection, |m| {
        //         m.menu("Open", Box::new(OpenEntry))
        //             .when(single_selection, |m| m.menu("Preview", Box::new(PreviewEntry)))
        //             .menu("Extract", Box::new(ExtractSelected))
        //             .menu("Test", Box::new(TestSelected))
        //             .separator()
        //             .menu("Rename", Box::new(RenameEntry))
        //             .menu("Delete", Box::new(DeleteSelected))
        //             .separator()
        //             .item(PopupMenuItem::submenu(
        //                 "Checksum",
        //                 PopupMenu::build(_window, cx, |menu, _window, _cx| {
        //                     menu.menu("CRC32", Box::new(ChecksumCRC32))
        //                         .menu("MD5", Box::new(ChecksumMD5))
        //                         .menu("SHA-1", Box::new(ChecksumSHA1))
        //                         .menu("SHA-256", Box::new(ChecksumSHA256))
        //                 }),
        //             ))
        //     })
        //     .separator()
        //     .menu("Select All", Box::new(SelectAll))
        //     .when(has_selection, |m| m.menu("Clear Selection", Box::new(ClearSelection)))
        //     .separator()
        //     .when(is_ready, |m| m.menu("Refresh", Box::new(Refresh)))
        //     .menu("Properties", Box::new(ShowProperties))
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
                    format!(
                        "{:.0}%",
                        (1.0 - (row.compressed_size as f64 / row.size as f64)) * 100.
                    )
                };
                ratio
            }
            "date" => {
                let date = row
                    .modified
                    .map(|t| t.naive_local().to_string())
                    .unwrap_or_default();
                date
            }
            _ => "".to_string(),
        }
    }

    // fn render_empty(&mut self, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    //     empty_view(cx, "")
    // }
    //
    // fn render_loading(&mut self, _size: gpui_component::Size, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    //     loading_view(cx)
    // }
}
