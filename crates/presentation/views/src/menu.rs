use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::AppMenuBar;
use gpui_component::{Icon, IconName, Sizable, TitleBar};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = menu_actions, no_json)]
struct OpenRecentArchive {
    path: SharedString,
}

actions!(
    menu_actions,
    [
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
        ToggleSidebar,
        SaveArchive,
        UndoArchive,
        RedoArchive,
        ExtractArchive,
    ]
);

pub fn build_menus(
    is_open: bool,
    has_selection: bool,
    recent_files: Vec<PathBuf>,
) -> Vec<gpui::Menu> {
    vec![
        gpui::Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open Archive", OpenArchive),
                MenuItem::submenu(gpui::Menu {
                    name: "Open Recent Archive".into(),
                    items: recent_files
                        .iter()
                        .map(|x| {
                            MenuItem::action(
                                SharedString::new(
                                    x.as_path().file_name().unwrap().to_string_lossy(),
                                ),
                                OpenRecentArchive {
                                    path: SharedString::from(
                                        x.as_path()
                                            .as_os_str()
                                            .to_os_string()
                                            .to_string_lossy()
                                            .to_string(),
                                    ),
                                },
                            )
                        })
                        .collect(),
                    disabled: false,
                }),
                MenuItem::action("Create Archive", CreateArchive),
                MenuItem::action("Add Files", AddFiles).disabled(!is_open),
                MenuItem::Separator,
                MenuItem::submenu(gpui::Menu {
                    name: "Test".into(),
                    items: vec![
                        MenuItem::action("Test Selected Files", TestSelected)
                            .disabled(!has_selection),
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
            items: vec![MenuItem::action("About", About)],
            disabled: false,
        },
    ]
}

#[derive(IntoElement)]
pub struct MenuView {
    pub bar: Entity<AppMenuBar>,
    pub sidebar_collapsed: bool,
}

impl MenuView {
    pub fn new(bar: Entity<AppMenuBar>, sidebar_collapsed: bool) -> Self {
        Self { bar, sidebar_collapsed }
    }
}

impl RenderOnce for MenuView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon = if self.sidebar_collapsed {
            IconName::PanelLeftOpen
        } else {
            IconName::PanelLeftClose
        };
        TitleBar::new()
            .child(
                div().flex().child(
                    Button::new("menu-toggle-sidebar")
                        .ghost()
                        .small()
                        .icon(Icon::new(icon).size_4())
                        .on_click(move |_, window, cx| {
                            window.dispatch_action(Box::new(ToggleSidebar), cx);
                        }),
                ),
            )
            .child(div().flex().items_center().child(self.bar.clone()))
    }
}
