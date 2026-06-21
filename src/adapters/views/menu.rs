use crate::adapters::events::ChecksumAlgorithm;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{AppMenuBar, DropdownMenu, PopupMenuItem};
use gpui_component::TitleBar;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuIntent {
    OpenArchive,
    CreateArchive,
    AddFiles,
    TestSelected,
    TestAll,
    CloseArchive,
    ShowProperties,
    SelectAll,
    InvertSelection,
    DeleteSelected,
    RenameSelected,
    Checksum(ChecksumAlgorithm),
    ShowSettings,
    About,
}

impl EventEmitter<MenuIntent> for Menu {}

pub struct Menu {
    is_open: bool,
    has_selection: bool,
    single_selection: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self { is_open: false, has_selection: false, single_selection: false }
    }

    pub fn set_state(&mut self, is_open: bool, has_selection: bool, single_selection: bool) {
        self.is_open = is_open;
        self.has_selection = has_selection;
        self.single_selection = single_selection;
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let self_handle = cx.entity();

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
                    let h = self_handle.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("Open Archive").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::OpenArchive)); }
                        }))
                        .item(PopupMenuItem::new("Create Archive").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::CreateArchive)); }
                        }))
                        .item(PopupMenuItem::new("Add Files").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::AddFiles)); }
                        }))
                        .separator()
                        .submenu("Test Archive", _window, _cx, {
                            let h = h.clone();
                            move |sub, _w, _c| {
                                sub.item(PopupMenuItem::new("Test Selected Files").on_click({
                                    let h = h.clone();
                                    move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::TestSelected)); }
                                }))
                                .item(PopupMenuItem::new("Test Entire Archive").on_click({
                                    let h = h.clone();
                                    move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::TestAll)); }
                                }))
                            }
                        })
                        .separator()
                        .item(PopupMenuItem::new("Close Archive").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::CloseArchive)); }
                        }))
                        .separator()
                        .item(PopupMenuItem::new("Properties").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::ShowProperties)); }
                        }))
                    }
                }))
                .child(Button::new("menu-edit").label("Edit").ghost().dropdown_menu({
                    let h = self_handle.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("Select All").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::SelectAll)); }
                        }))
                        .item(PopupMenuItem::new("Invert Selection").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::InvertSelection)); }
                        }))
                        .separator()
                        .item(PopupMenuItem::new("Delete").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::DeleteSelected)); }
                        }))
                        .item(PopupMenuItem::new("Rename").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::RenameSelected)); }
                        }))
                    }
                }))
                .child(Button::new("menu-tools").label("Tools").ghost().dropdown_menu({
                    let h = self_handle.clone();
                    move |menu, window, cx| {
                        let h_sub = h.clone();
                        menu.submenu("Checksum", window, cx, move |sub, _w, _c| {
                            let h_crc32 = h_sub.clone();
                            let h_md5 = h_sub.clone();
                            let h_sha1 = h_sub.clone();
                            let h_sha256 = h_sub.clone();
                            sub.item(PopupMenuItem::new("CRC32").on_click({
                                move |_, _, cx| { h_crc32.update(cx, |_, cx| cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Crc32))); }
                            }))
                            .item(PopupMenuItem::new("MD5").on_click({
                                move |_, _, cx| { h_md5.update(cx, |_, cx| cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Md5))); }
                            }))
                            .item(PopupMenuItem::new("SHA1").on_click({
                                move |_, _, cx| { h_sha1.update(cx, |_, cx| cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha1))); }
                            }))
                            .item(PopupMenuItem::new("SHA256").on_click({
                                move |_, _, cx| { h_sha256.update(cx, |_, cx| cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha256))); }
                            }))
                        })
                        .separator()
                        .item(PopupMenuItem::new("Settings").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::ShowSettings)); }
                        }))
                    }
                }))
                .child(Button::new("menu-help").label("Help").ghost().dropdown_menu({
                    let h = self_handle.clone();
                    move |menu, _window, _cx| {
                        menu.item(PopupMenuItem::new("About bit7z Archiver").on_click({
                            let h = h.clone();
                            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(MenuIntent::About)); }
                        }))
                    }
                })),
        )
    }
}
