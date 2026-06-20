use crate::application::events::ArchiveVmEvent;
use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::adapters::views::components::state_view::{empty_view, error_view, loading_view};
use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::table::{Column as TableColumn, ColumnSort, DataTable, TableDelegate, TableState};
use humansize::{format_size, BINARY};

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
            .map(|e| e.original_index as usize)
            .map(|idx| vm.selection.contains(&(idx as u32)))
            .unwrap_or(false);
        let theme = cx.global::<Theme>();
        drop(vm);

        let avm = self.archive_vm.clone();
        let avm_r = self.archive_vm.clone();
        div().id(("row", row_ix))
            .cursor_pointer()
            .when(selected, |d| d.bg(theme.selection))
            .when(!selected && row_ix % 2 == 0, |d| d.bg(theme.surface))
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                avm.update(cx, |vm, cx| vm.handle_level_click(row_ix, &event.modifiers, cx));
            })
            .on_mouse_down(MouseButton::Right, move |_: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                avm_r.update(cx, |vm, cx| {
                    if let Some(entry) = vm.level_entries.get(row_ix) {
                        if !vm.selection.contains(&entry.original_index) {
                            vm.select(row_ix as u32, &Modifiers::none(), cx);
                        }
                    }
                });
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

                let table_entity = self.table_state.clone();
                let avm = archive_vm.clone();
                container.child(
                    div().flex_1().child(DataTable::new(&table_entity).stripe(false).bordered(false))
                        .id("entry-table-area")
                        .context_menu(move |menu, window, cx| {
                            let current_vm = avm.read(cx);
                            let has_selection = !current_vm.selection.is_empty();
                            let single_selection = current_vm.selection.len() == 1;
                            let is_ready = matches!(current_vm.status, ViewStatus::Ready);
                            drop(current_vm);

                            let mut m = menu;
                            let vm_open = avm.clone();
                            if has_selection {
                                m = m.item(
                                    PopupMenuItem::new("Open")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_open.update(cx, |vm, cx| vm.open_entry(cx));
                                        })
                                );
                            }
                            let vm_preview = avm.clone();
                            if single_selection {
                                m = m.item(
                                    PopupMenuItem::new("Preview")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_preview.update(cx, |vm, cx| vm.preview_entry(cx));
                                        })
                                );
                            }
                            let vm_extract = avm.clone();
                            if has_selection {
                                m = m.item(
                                    PopupMenuItem::new("Extract...")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_extract.update(cx, |vm, cx| vm.request_extract(cx));
                                        })
                                );
                            }
                            m = m.separator();
                            let vm_rename = avm.clone();
                            if single_selection {
                                m = m.item(
                                    PopupMenuItem::new("Rename")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_rename.update(cx, |vm, cx| {
                                                if let Some(idx) = vm.first_selected_index() {
                                                    cx.emit(crate::application::events::ArchiveVmEvent::RequestRename {
                                                        index: idx,
                                                        new_name: String::new(),
                                                    });
                                                }
                                            });
                                        })
                                );
                            }
                            let vm_delete = avm.clone();
                            if has_selection {
                                m = m.item(
                                    PopupMenuItem::new("Delete")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_delete.update(cx, |vm, cx| vm.delete_selected(cx));
                                        })
                                );
                            }
                            if has_selection {
                                m = m.separator();
                                let avm_ck = avm.clone();
                                m = m.submenu("Checksum", window, cx, move |menu, _, _| {
                                    let avm_crc32 = avm_ck.clone();
                                    let avm_md5 = avm_ck.clone();
                                    let avm_sha1 = avm_ck.clone();
                                    let avm_sha256 = avm_ck.clone();
                                    menu.item(PopupMenuItem::new("CRC32").on_click({
                                        let avm = avm_crc32.clone();
                                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            avm.update(cx, |vm, cx| vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Crc32));
                                        }
                                    }))
                                    .item(PopupMenuItem::new("MD5").on_click({
                                        let avm = avm_md5.clone();
                                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            avm.update(cx, |vm, cx| vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Md5));
                                        }
                                    }))
                                    .item(PopupMenuItem::new("SHA1").on_click({
                                        let avm = avm_sha1.clone();
                                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            avm.update(cx, |vm, cx| vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha1));
                                        }
                                    }))
                                    .item(PopupMenuItem::new("SHA256").on_click({
                                        let avm = avm_sha256.clone();
                                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            avm.update(cx, |vm, cx| vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha256));
                                        }
                                    }))
                                });
                            }
                            m = m.separator();
                            let vm_all = avm.clone();
                            m = m.item(
                                PopupMenuItem::new("Select All")
                                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                        vm_all.update(cx, |vm, cx| vm.select_all(cx));
                                    })
                            );
                            let vm_clear = avm.clone();
                            if has_selection {
                                m = m.item(
                                    PopupMenuItem::new("Clear Selection")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_clear.update(cx, |vm, cx| vm.clear_selection(cx));
                                        })
                                );
                            }
                            m = m.separator();
                            let vm_refresh = avm.clone();
                            if is_ready {
                                m = m.item(
                                    PopupMenuItem::new("Refresh")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_refresh.update(cx, |vm, cx| vm.refresh(cx));
                                        })
                                );
                            }
                            let vm_props = avm.clone();
                            if single_selection {
                                m = m.item(
                                    PopupMenuItem::new("Properties")
                                        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            vm_props.update(cx, |vm, cx| vm.show_properties(cx));
                                        })
                                );
                            }
                            m
                        })
                )
            }
        }
    }
}
