use std::path::PathBuf;
use gpui::*;
use gpui_component::menu::AppMenuBar;
use gpui_component::{GlobalState, Icon, IconName, TitleBar};
use gpui_component::dock::PanelEvent;
use gpui_component::sidebar::{SidebarCollapsible, SidebarToggleButton};
use serde::Deserialize;
use crate::adapters::events::ArchiveVmEvent;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = menu_actions, no_json)]
struct OpenRecentArchive {
    path: SharedString
}

actions!(menu_actions, [
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

pub struct Menu {
    bar: Entity<AppMenuBar>,
    expanded_sidebar: bool,
}

impl Menu {
    pub fn new(expanded_sidebar:bool,cx: &mut Context<Self>) -> Self {
        let bar = AppMenuBar::new(cx);
        let menu = Self { expanded_sidebar, bar };
        menu.reload(cx);
        menu
    }

    pub fn set_state(&mut self, is_open: bool, has_selection: bool, _single_selection: bool, cx: &mut Context<Self>) {
        let menus = Self::build_menus(is_open, has_selection, vec![]);
        let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
        GlobalState::global_mut(cx).set_app_menus(owned);
        self.bar.update(cx, |bar, cx| bar.reload(cx));
    }

    fn reload(&self, cx: &mut Context<Self>) {
        let menus = Self::build_menus(false, false,vec![]);
        let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
        GlobalState::global_mut(cx).set_app_menus(owned);
        self.bar.update(cx, |bar, cx| bar.reload(cx));
    }

    fn build_menus(is_open: bool, has_selection: bool,recent_files: Vec<PathBuf>) -> Vec<gpui::Menu> {
        vec![
            gpui::Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("Open Archive", OpenArchive),
                    MenuItem::submenu(gpui::Menu {
                        name: "Open Recent Archive".into(),
                        items: recent_files.iter().map(|x| {
                            MenuItem::action(SharedString::new(x.as_path().file_name().unwrap().to_string_lossy()),OpenRecentArchive { path: SharedString::from(x.as_path().as_os_str().to_os_string().to_string_lossy().to_string()) })
                        }).collect(),
                        disabled: false,
                    }),
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
                    MenuItem::action("About", About),
                ],
                disabled: false,
            },
        ]
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .child(div().flex()
                .child(SidebarToggleButton::new()
                    .collapsed(self.expanded_sidebar)
                    .on_click(|click_event, w, cx| {
                        // cx.new(
                        //     |cx| {
                        //         cx.emit(ArchiveVmEvent::RequestShowSettings)
                        //     }
                        // );
                    })
                )
            )
            .child(div().flex().items_center().child(self.bar.clone()))
    }
}
