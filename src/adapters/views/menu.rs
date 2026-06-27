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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let bar = AppMenuBar::new(cx);

        macro_rules! bind_action {
            ($action:ty, $intent:expr) => {
                cx.on_action(
                    std::any::TypeId::of::<$action>(),
                    window,
                    move |_: &mut Self, _: &dyn std::any::Any, _: DispatchPhase, _: &mut Window, cx: &mut Context<Self>| {
                        cx.emit($intent);
                    },
                );
            };
        }

        bind_action!(OpenArchive, MenuIntent::OpenArchive);
        bind_action!(CreateArchive, MenuIntent::CreateArchive);
        bind_action!(AddFiles, MenuIntent::AddFiles);
        bind_action!(TestSelected, MenuIntent::TestSelected);
        bind_action!(TestAll, MenuIntent::TestAll);
        bind_action!(CloseArchive, MenuIntent::CloseArchive);
        bind_action!(ShowProperties, MenuIntent::ShowProperties);
        bind_action!(SelectAll, MenuIntent::SelectAll);
        bind_action!(InvertSelection, MenuIntent::InvertSelection);
        bind_action!(DeleteSelected, MenuIntent::DeleteSelected);
        bind_action!(RenameSelected, MenuIntent::RenameSelected);
        bind_action!(ShowSettings, MenuIntent::ShowSettings);
        bind_action!(About, MenuIntent::About);

        // Checksum actions — each maps to Checksum(intent) with its algorithm
        cx.on_action(
            std::any::TypeId::of::<ChecksumCrc32>(),
            window,
            move |_: &mut Self, _: &dyn std::any::Any, _: DispatchPhase, _: &mut Window, cx: &mut Context<Self>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Crc32));
            },
        );
        cx.on_action(
            std::any::TypeId::of::<ChecksumMd5>(),
            window,
            move |_: &mut Self, _: &dyn std::any::Any, _: DispatchPhase, _: &mut Window, cx: &mut Context<Self>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Md5));
            },
        );
        cx.on_action(
            std::any::TypeId::of::<ChecksumSha1>(),
            window,
            move |_: &mut Self, _: &dyn std::any::Any, _: DispatchPhase, _: &mut Window, cx: &mut Context<Self>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha1));
            },
        );
        cx.on_action(
            std::any::TypeId::of::<ChecksumSha256>(),
            window,
            move |_: &mut Self, _: &dyn std::any::Any, _: DispatchPhase, _: &mut Window, cx: &mut Context<Self>| {
                cx.emit(MenuIntent::Checksum(ChecksumAlgorithm::Sha256));
            },
        );

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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(div().flex().items_center().child(self.bar.clone()))
    }
}
