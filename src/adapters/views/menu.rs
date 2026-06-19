use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::application::events::ArchiveVmEvent;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
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
        drop(vm);

        let vm_entity = self.archive_vm.clone();

        TitleBar::new().child(
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
                        .item(PopupMenuItem::new("Open").disabled(!has_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.open_entry(cx))
                        }))
                        .item(PopupMenuItem::new("View").disabled(!has_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.preview_entry(cx))
                        }))
                        .item(PopupMenuItem::new("Edit").disabled(!has_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.edit_entry(cx))
                        }))
                        .separator()
                        .item(PopupMenuItem::new("New Folder").disabled(!is_open).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_new_folder(cx))
                        }))
                        .item(PopupMenuItem::new("New File").disabled(!is_open).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_new_file(cx))
                        }))
                        .separator()
                        .item(PopupMenuItem::new("Close Archive").disabled(!is_open).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.close(cx))
                        }))
                        .separator()
                        .item(PopupMenuItem::new("Properties").disabled(!has_selection).on_click({
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
                        .item(PopupMenuItem::new("Copy").disabled(!has_selection))
                        .item(PopupMenuItem::new("Cut").disabled(!has_selection))
                        .item(PopupMenuItem::new("Paste").disabled(!is_open))
                        .separator()
                        .item(PopupMenuItem::new("Delete").disabled(!has_selection).on_click({
                            let vm = vm.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.delete_selected(cx))
                        }))
                        .item(PopupMenuItem::new("Rename").disabled(!has_selection).on_click({
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
                .child(Button::new("menu-view").label("View").ghost().dropdown_menu(
                    |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("Large Icons"))
                            .item(PopupMenuItem::new("Small Icons"))
                            .item(PopupMenuItem::new("List"))
                            .item(PopupMenuItem::new("Details"))
                            .separator()
                            .item(PopupMenuItem::new("Flat View"))
                            .separator()
                            .item(PopupMenuItem::new("Show Toolbar"))
                            .item(PopupMenuItem::new("Show Status Bar"))
                            .item(PopupMenuItem::new("Show Preview Panel"))
                            .item(PopupMenuItem::new("Show Directory Tree"))
                    },
                ))
                .child(Button::new("menu-tools").label("Tools").ghost().dropdown_menu({
                    let vm = vm_entity.clone();
                    move |menu, window, cx| {
                        let vm_sub = vm.clone();
                        menu.submenu("Checksum", window, cx, move |sub, _w, _c| {
                            let vm = vm_sub.clone();
                            sub.item(PopupMenuItem::new("CRC32").disabled(!has_selection).on_click({
                                let vm = vm.clone();
                                move |_, _, cx| {
                                    vm.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Crc32)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("MD5").disabled(!has_selection).on_click({
                                let vm = vm.clone();
                                move |_, _, cx| {
                                    vm.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Md5)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("SHA1").disabled(!has_selection).on_click({
                                let vm = vm.clone();
                                move |_, _, cx| {
                                    vm.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha1)
                                    });
                                }
                            }))
                            .item(PopupMenuItem::new("SHA256").disabled(!has_selection).on_click({
                                let vm = vm.clone();
                                move |_, _, cx| {
                                    vm.update(cx, |vm, cx| {
                                        vm.request_checksum(cx, crate::application::events::ChecksumAlgorithm::Sha256)
                                    });
                                }
                            }))
                        })
                        .separator()
                        .item(PopupMenuItem::new("Settings").on_click({
                            let vm = vm_entity.clone();
                            move |_, _, cx| vm.update(cx, |vm, cx| vm.request_show_settings(cx))
                        }))
                    }
                }))
                .child(
                    Button::new("menu-favorites")
                        .label("Favorites")
                        .ghost()
                        .dropdown_menu(move |menu, _window, _cx| {
                            menu.item(PopupMenuItem::new("Add to Favorites").disabled(!is_open))
                                .item(PopupMenuItem::new("Organize Favorites"))
                        }),
                )
                .child(
                    Button::new("menu-help")
                        .label("Help")
                        .ghost()
                        .dropdown_menu(|menu, _window, _cx| {
                            menu.item(PopupMenuItem::new("About"))
                        }),
                ),
        )
    }
}
