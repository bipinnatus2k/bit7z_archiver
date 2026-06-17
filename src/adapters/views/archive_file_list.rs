use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::adapters::views::components::state_view::{empty_view, error_view, loading_view};
use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::table::{Column as TableColumn, ColumnSort, DataTable, TableDelegate, TableState};
use humansize::{format_size, BINARY};
use std::rc::Rc;

struct FileTableDelegate {
    archive_vm: Entity<ArchiveViewModel>,
}

impl TableDelegate for FileTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize { 5 }

    fn rows_count(&self, cx: &App) -> usize {
        self.archive_vm.read(cx).displayed_entries().len()
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
        let avm = self.archive_vm.clone();
        avm.update(cx, |vm, cx| {
            vm.apply_sort(col_ix as u32, sort == ColumnSort::Ascending);
            cx.notify();
        });
    }

    fn render_tr(&mut self, row_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> Stateful<Div> {
        let vm = self.archive_vm.read(cx);
        let entries = vm.displayed_entries();
        let selected = entries.get(row_ix)
            .and_then(|e| e.original_idx)
            .map(|idx| vm.selection.contains(&(idx as u32)))
            .unwrap_or(false);
        let theme = cx.global::<Theme>();
        drop(vm);

        let avm = self.archive_vm.clone();
        div().id(("row", row_ix))
            .cursor_pointer()
            .when(selected, |d| d.bg(theme.selection))
            .when(!selected && row_ix % 2 == 0, |d| d.bg(theme.surface))
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                avm.update(cx, |vm, cx| vm.handle_level_click(row_ix, &event.modifiers, cx));
            })
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let entries = vm.displayed_entries();
        let entry = entries.get(row_ix);

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
    pub archive_vm: Entity<ArchiveViewModel>,
    table_state: Entity<TableState<FileTableDelegate>>,
}

impl ArchiveFileList {
    pub fn new(archive_vm: Entity<ArchiveViewModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = FileTableDelegate { archive_vm: archive_vm.clone() };
        let table_state = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .row_selectable(false)
                .cell_selectable(false)
        });
        Self { archive_vm, table_state }
    }
}

impl Render for ArchiveFileList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let theme = cx.global::<Theme>();

        let base = gpui_component::v_flex()
            .flex_1()
            .border_b_1()
            .border_color(theme.border);

        match &vm.status {
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
                let entries = vm.displayed_entries();
                let is_root = vm.current_path.is_empty();
                let path_str = vm.current_path.trim_end_matches('/').to_string();
                let archive_vm = self.archive_vm.clone();
                drop(vm);

                // Navigation path bar
                let mut container = base;
                if !is_root {
                    let avm = archive_vm.clone();
                    container = container
                        .child(
                            div().flex().flex_row().gap_1().px_2().py_1()
                                .bg(theme.surface)
                                .child(
                                    div().cursor_pointer().child(" \u{2190} ")
                                        .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _: &mut Window, cx: &mut App| {
                                            avm.update(cx, |vm, cx| { vm.navigate_up(cx); });
                                        })
                                )
                                .child(format!(" \u{1f4c2} {}", path_str))
                        );
                }

                // Refresh TableState to sync with current VM state
                let table_entity = self.table_state.clone();
                container.child(
                    div().flex_1().child(DataTable::new(&table_entity).stripe(false).bordered(false))
                )
            }
        }
    }
}
