use crate::adapters::events::ChecksumAlgorithm;
use gpui::*;
use gpui_component::menu::AppMenuBar;
use gpui_component::{GlobalState, TitleBar};

gpui::actions!(menu_actions, [
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
    ChecksumCrc32,
    ChecksumMd5,
    ChecksumSha1,
    ChecksumSha256,
    ShowSettings,
    About,
]);

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
    bar: Entity<AppMenuBar>,
}

impl Menu {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let bar = AppMenuBar::new(cx);
        let menu = Self { bar };
        menu.reload(cx);
        menu
    }

    pub fn set_state(&mut self, is_open: bool, has_selection: bool, _single_selection: bool, cx: &mut Context<Self>) {
        let menus = Self::build_menus(is_open, has_selection);
        let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
        GlobalState::global_mut(cx).set_app_menus(owned);
        self.bar.update(cx, |bar, cx| bar.reload(cx));
    }

    fn reload(&self, cx: &mut Context<Self>) {
        let menus = Self::build_menus(false, false);
        let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
        GlobalState::global_mut(cx).set_app_menus(owned);
        self.bar.update(cx, |bar, cx| bar.reload(cx));
    }

    fn build_menus(is_open: bool, has_selection: bool) -> Vec<gpui::Menu> {
        vec![
            gpui::Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("Open Archive", OpenArchive),
                    MenuItem::action("Create Archive", CreateArchive),
                    MenuItem::action("Add Files", AddFiles).disabled(!is_open),
                    MenuItem::Separator,
                    MenuItem::submenu(gpui::Menu {
                        name: "Test".into(),
                        items: vec![
                            MenuItem::action("Test Selected Files", TestSelected).disabled(!has_selection),
                            MenuItem::action("Test Entire Archive", TestAll).disabled(!is_open),
                        ],
                        disabled: !is_open,
                    }),
                    MenuItem::Separator,
                    MenuItem::action("Close Archive", CloseArchive).disabled(!is_open),
                    MenuItem::Separator,
                    MenuItem::action("Properties", ShowProperties).disabled(!is_open),
                ],
                disabled: false,
            },
            gpui::Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Select All", SelectAll).disabled(!is_open),
                    MenuItem::action("Invert Selection", InvertSelection).disabled(!is_open),
                    MenuItem::Separator,
                    MenuItem::action("Delete", DeleteSelected).disabled(!has_selection),
                    MenuItem::action("Rename", RenameSelected).disabled(!has_selection),
                ],
                disabled: false,
            },
            gpui::Menu {
                name: "Tools".into(),
                items: vec![
                    MenuItem::submenu(gpui::Menu {
                        name: "Checksum".into(),
                        items: vec![
                            MenuItem::action("CRC32", ChecksumCrc32).disabled(!has_selection),
                            MenuItem::action("MD5", ChecksumMd5).disabled(!has_selection),
                            MenuItem::action("SHA1", ChecksumSha1).disabled(!has_selection),
                            MenuItem::action("SHA256", ChecksumSha256).disabled(!has_selection),
                        ],
                        disabled: !has_selection,
                    }),
                    MenuItem::Separator,
                    MenuItem::action("Settings", ShowSettings),
                ],
                disabled: false,
            },
            gpui::Menu {
                name: "Help".into(),
                items: vec![
                    MenuItem::action("About bit7z Archiver", About),
                ],
                disabled: false,
            },
        ]
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_action(cx.listener(|_: &mut Menu, _: &OpenArchive, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::OpenArchive);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &CreateArchive, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::CreateArchive);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &AddFiles, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::AddFiles);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &TestSelected, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::TestSelected);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &TestAll, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::TestAll);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &CloseArchive, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::CloseArchive);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ShowProperties, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::ShowProperties);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &SelectAll, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::SelectAll);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &InvertSelection, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::InvertSelection);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &DeleteSelected, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::DeleteSelected);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &RenameSelected, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::RenameSelected);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ChecksumCrc32, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Crc32));
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ChecksumMd5, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Md5));
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ChecksumSha1, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha1));
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ChecksumSha256, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha256));
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &ShowSettings, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::ShowSettings);
            }))
            .on_action(cx.listener(|_: &mut Menu, _: &About, _: &mut Window, cx: &mut Context<Menu>| {
                cx.emit(MenuIntent::About);
            }))
            .child(TitleBar::new()
                .child(div().flex().items_center().child(self.bar.clone())))
    }
}
