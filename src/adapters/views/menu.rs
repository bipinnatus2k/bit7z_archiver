use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::application::events::ArchiveVmEvent;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{AppMenuBar, DropdownMenu, PopupMenuItem};
use gpui_component::TitleBar;

pub struct Menu {
    archive_vm: Entity<ArchiveViewModel>,
}

impl Menu {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let is_open = vm.archive.is_some();
        let has_selection = !vm.selection.is_empty();
        let single_selection = vm.selection.len() == 1;

        let vm_entity = self.archive_vm.clone();
        drop(vm);

        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(AppMenuBar::new(cx))
            )
            .child(
            div()
                .flex()
                .items_center()
                .gap_0()
                .child(Button::new("menu-file").label("File").ghost().dropdown_menu({
                    let vm = vm_entity.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("Open Archive").on_click({
                            let vm = vm.clone();
                            move |_, _, cx| {
                                if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                    vm.update(cx, |vm, cx| vm.open_archive(&path, None, cx));
                                }
                            }
                        }))
                        .item(PopupMenuItem::new("Create Archive").on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_create(cx))
                        }))
                        .item(PopupMenuItem::new("Add Files").disabled(!is_open).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_add_files(cx))
                        }))
                        .separator()
                        .submenu("Test Archive", _window, _cx, {
                            let vm = vm.clone();
                            move |sub, _w, _c| {
                                sub.item(PopupMenuItem::new("Test Selected Files").on_click({
                                    let vm = vm.clone();
                                    move |_, _, cx| vm.update(cx, |vm, cx| vm.test_selected(cx))
                                }))
                                .item(PopupMenuItem::new("Test Entire Archive").on_click({
                                    let vm = vm.clone();
                                    move |_, _, cx| vm.update(cx, |vm, cx| vm.test_all(cx))
                                }))
                            }
                        })
                        .separator()
                        .item(PopupMenuItem::new("Close Archive").disabled(!is_open).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.close(cx))
                        }))
                        .separator()
                        .item(PopupMenuItem::new("Properties").disabled(!single_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.show_properties(cx))
                        }))
                    }
                }))
                .child(Button::new("menu-edit").label("Edit").ghost().dropdown_menu({
                    let vm = vm_entity.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("Select All").on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.select_all(cx))
                        }))
                        .item(
                            PopupMenuItem::new("Invert Selection")
                                .disabled(!has_selection)
                                .on_click({
                                    let vm = vm.clone();
                                    move |_, _, cx| vm.update(cx, |vm, cx| vm.invert_selection(cx))
                                }),
                        )
                        .separator()
                        .item(PopupMenuItem::new("Delete").disabled(!has_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.delete_selected(cx))
                        }))
                        .item(PopupMenuItem::new("Rename").disabled(!single_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| {
                                vm.update(cx, |vm, cx| {
                                    if let Some(idx) = vm.first_selected_index() {
                                        cx.emit(ArchiveVmEvent::RequestRename {
                                            index: idx,
                                            new_name: String::new(),
                                        });
                                    }
                                });
                            }
                        }))
                    }
                }))
                .child(Button::new("menu-tools").label("Tools").ghost().dropdown_menu({
                    let vm = vm_entity.clone();
                    move |menu, window, cx| {
                        let vm_sub = vm.clone();
                        menu.submenu("Checksum", window, cx, move |sub, _w, _c| {
                            let vm_crc32 = vm_sub.clone();
                            let vm_md5 = vm_sub.clone();
                            let vm_sha1 = vm_sub.clone();
                            let vm_sha256 = vm_sub.clone();
                            sub.item(PopupMenuItem::new("CRC32").disabled(!has_selection).on_click({
                                move |_, _, cx| {
                                    vm_crc32.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Crc32)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("MD5").disabled(!has_selection).on_click({
                                move |_, _, cx| {
                                    vm_md5.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Md5)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("SHA1").disabled(!has_selection).on_click({
                                move |_, _, cx| {
                                    vm_sha1.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha1)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("SHA256").disabled(!has_selection).on_click({
                                move |_, _, cx| {
                                    vm_sha256.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha256)
                                    });
                                }
                            }))
                        })
                        .separator()
                        .item(PopupMenuItem::new("Settings").on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_show_settings(cx))
                        }))
                    }
                }))
                .child(Button::new("menu-help").label("Help").ghost().dropdown_menu({
                    let vm = vm_entity.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("About bit7z Archiver").on_click({
                            let vm = vm.clone();
                            move |_, _, cx| log::info!("bit7z Archiver {}", env!("CARGO_PKG_VERSION"))
                        }))
                    }
                })),
        )
    }
}
