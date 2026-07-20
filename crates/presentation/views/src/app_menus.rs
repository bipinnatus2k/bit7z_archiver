use crate::{About, Benchmark, Checksum, CleanRecentFiles, CloseArchive, CreateArchive, DebugInfo, Open, OpenArchive, OpenSettings, Quit, RequestAddFiles, SaveAs, SelectLocale, ShowProperties, TestAll, TestSelected, ToggleSearch};
use bit7z_domain::checksum::ChecksumAlgorithm;
use bit7z_pres_theme::theme::{SwitchTheme, SwitchThemeMode};
use gpui::{App, Entity, Menu, MenuItem, SharedString};
use gpui_component::{
    menu::AppMenuBar, ActiveTheme as _, GlobalState, Theme, ThemeMode, ThemeRegistry,
};

pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    let app_menu_bar = AppMenuBar::new(cx);
    let title: SharedString = title.into();
    let is_open = false;
    update_app_menu(title.clone(), is_open,app_menu_bar.clone(), cx);

    cx.on_action({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |s: &SelectLocale, cx: &mut App| {
            rust_i18n::set_locale(&s.0.as_str());
            update_app_menu(title.clone(),is_open, app_menu_bar.clone(), cx);
        }
    });

    // Observe theme changes to update the menu to refresh the checked state
    cx.observe_global::<Theme>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(),is_open, app_menu_bar.clone(), cx);
        }
    })
    .detach();

    app_menu_bar
}

fn update_app_menu(
    title: impl Into<SharedString>,
    is_open: bool,
    app_menu_bar: Entity<AppMenuBar>,
    cx: &mut App
) {
    let title: SharedString = title.into();

    cx.set_menus(build_menus(title.clone(), is_open, cx));
    let menus = build_menus(title,is_open,cx)
        .into_iter()
        .map(|menu| menu.owned())
        .collect();
    GlobalState::global_mut(cx).set_app_menus(menus);

    app_menu_bar.update(cx, |menu_bar, cx| {
        menu_bar.reload(cx);
    })
}

fn build_menus(title: impl Into<SharedString>, is_open:bool, cx: &App) -> Vec<Menu> {
    vec![
        Menu {
            name: title.into(),
            items: vec![
                MenuItem::Submenu(Menu {
                    name: "Appearance".into(),
                    items: vec![
                        MenuItem::action("Light", SwitchThemeMode(ThemeMode::Light))
                            .checked(!cx.theme().mode.is_dark()),
                        MenuItem::action("Dark", SwitchThemeMode(ThemeMode::Dark))
                            .checked(cx.theme().mode.is_dark()),
                    ],
                    disabled: false,
                }),
                theme_menu(cx),
                language_menu(cx),
                MenuItem::Separator,
                MenuItem::action("Settings",OpenSettings),
                MenuItem::Separator,
                MenuItem::submenu(Menu {
                    name: "Help".into(),
                    items: vec![
                        MenuItem::action("Debug Info", DebugInfo),
                        MenuItem::action("About", About),
                    ],
                    disabled: false,
                }),
                MenuItem::Separator,
                MenuItem::action("Quit", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open...", OpenArchive()),
                recent_menu(cx),
                MenuItem::Separator,
                MenuItem::action("Save as...",SaveAs),
                MenuItem::Separator,
                MenuItem::action("Create Archive", CreateArchive()),
                MenuItem::action("Add Files", RequestAddFiles()).disabled(!is_open),
                MenuItem::Separator,
                MenuItem::submenu(Menu {
                    name: "Test".into(),
                    items: vec![
                        MenuItem::action("Test Selected Files", TestSelected()),
                        MenuItem::action("Test Entire Archive", TestAll()).disabled(!is_open),
                    ],
                    disabled: !is_open,
                }),
                MenuItem::Separator,
                MenuItem::action("Close Archive", CloseArchive()).disabled(!is_open),
                MenuItem::Separator,
                MenuItem::action("Properties", ShowProperties()).disabled(!is_open),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Cut", gpui_component::input::Cut),
                MenuItem::action("Copy", gpui_component::input::Copy),
                MenuItem::action("Paste", gpui_component::input::Paste),
                MenuItem::separator(),
                MenuItem::action("Delete", gpui_component::input::Delete),
                MenuItem::separator(),
                MenuItem::action("Find", gpui_component::input::Search),
                MenuItem::separator(),
                MenuItem::action("Select All", gpui_component::input::SelectAll),
            ],
            disabled: false,
        },
        Menu {
            name: "Tools".into(),
            items: vec![
                MenuItem::submenu(Menu {
                    name: "Checksum".into(),
                    items: vec![
                        MenuItem::action("CRC32", Checksum(ChecksumAlgorithm::Crc32 )),
                        MenuItem::action("MD5", Checksum(ChecksumAlgorithm::Md5)),
                        MenuItem::action("SHA-1", Checksum(ChecksumAlgorithm::Sha1)),
                        MenuItem::action("SHA-256", Checksum(ChecksumAlgorithm::Sha256)),
                    ],
                    disabled:false,
                    // disabled: !has_selection,
                }),
                MenuItem::action("Search", ToggleSearch),
                MenuItem::action("Benchmark",Benchmark )
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![],
            disabled: false,
        },
    ]
}

fn language_menu(_: &App) -> MenuItem {
    let available_locales = rust_i18n::available_locales!();
    let current_locale = rust_i18n::locale().to_string();
    MenuItem::Submenu(Menu {
        name: "Language".into(),
        items: available_locales.iter().map(
            |locale| {
                MenuItem::action(locale.as_str(),SelectLocale(SharedString::from(locale.as_str()))).checked(locale.to_lowercase().to_string() == current_locale.to_lowercase())
            }
        ).collect(),
        disabled: false,
    })
}

fn theme_menu(cx: &App) -> MenuItem {
    let themes = ThemeRegistry::global(cx).sorted_themes();
    let current_name = cx.theme().theme_name();
    MenuItem::Submenu(Menu {
        name: "Theme".into(),
        items: themes
            .iter()
            .map(|theme| {
                let checked = current_name == &theme.name;
                MenuItem::action(theme.name.clone(), SwitchTheme(theme.name.clone()))
                    .checked(checked)
            })
            .collect(),
        disabled: false,
    })
}

fn recent_menu(cx: &App) -> MenuItem {
    MenuItem::submenu(Menu {
        name: "Open Recent Archive".into(),
        items: vec![
            MenuItem::action("Clean recent files", CleanRecentFiles)
        ],
        disabled: false,
    })
}
