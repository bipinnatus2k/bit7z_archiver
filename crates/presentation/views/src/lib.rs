#![feature(str_as_str)]
// Load I18n macro, for allow you use `t!` macro in anywhere.
#[macro_use]
extern crate rust_i18n;

use crate::archive_file_manager::ArchiveFileManager;
use crate::story_root::StoryRoot;
use crate::utils::window::create_new_window_with_size;
use bit7z_domain::checksum::ChecksumAlgorithm;
use gpui::{actions, div, px, size, Action, App, AppContext, Entity, Global, IntoElement, KeyBinding, ParentElement, SharedString, Styled, TextRenderingMode};
use gpui_component::text::Text;
use gpui_component::{ActiveTheme, Root, WindowExt};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use gpui_component::scroll::ScrollableElement;
use bit7z_domain::repository::ArchiveRepository;
use bit7z_pres_dialogs::about::AboutDialog;

// pub mod app_shell;
pub mod archive_sidebar;
// pub mod archive_file_list;
// pub mod menu;
// pub mod preview_panel;
// pub mod root;
pub mod file_list;
pub mod status_bar;
pub mod toolbar;
pub mod utils;
// mod story_container;
pub(crate) mod app_menus;
pub mod archive_file_manager;
pub(crate) mod story_root;
mod title_bar;
mod usecase;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct Extract();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct DeleteSelected();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct ShowProperties();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct TestSelected();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct TestAll();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct OpenEntry();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct ViewEntry();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct EditEntry();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct RequestNewFolder();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct RequestNewFile();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct Checksum(ChecksumAlgorithm);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct RequestAddFiles();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct OpenArchive {}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct CreateArchive();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct CloseArchive();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct Refresh();

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct ToggleViewMode {}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct ToggleFlatView {}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct ShowNotificationInfo {
    message: SharedString,
}

actions!(
    story,
    [
        About,
        Open,
        Quit,
        ToggleSearch,
        Benchmark,
        Tab,
        TabPrev,
        DebugInfo,
        CleanRecentFiles,
        SaveAs,
        OpenSettings
        // ShowPanelInfo,
               // ToggleListActiveHighlight
    ]
);

const PANEL_NAME: &str = "StoryContainer";

// pub struct AppState {
//     // pub invisible_panels: Entity<Vec<SharedString>>,
// }
// impl AppState {
//     fn init(cx: &mut App) {
//         let state = Self {
//             // invisible_panels: cx.new(|_| Vec::new()),
//         };
//         cx.set_global::<AppState>(state);
//     }
//
//     pub fn global(cx: &App) -> &Self {
//         cx.global::<Self>()
//     }
//
//     pub fn global_mut(cx: &mut App) -> &mut Self {
//         cx.global_mut::<Self>()
//     }
// }
//
// impl Global for AppState {}

fn render_mode_to_string(mode: TextRenderingMode) -> String {
    match mode {
        TextRenderingMode::PlatformDefault => "PlatformDefault".into(),
        TextRenderingMode::Subpixel => "Grayscale".into(),
        TextRenderingMode::Grayscale => "Subpixel".into(),
    }
}

pub fn init(cx: &mut App, repo: Arc<dyn ArchiveRepository>) {
    // Try to initialize tracing subscriber, but ignore if already initialized
    #[cfg(not(target_family = "wasm"))]
    {
        use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("gpui_component=trace".parse().unwrap()),
            )
            .try_init();
    }

    // For WASM, use a subscriber without time support
    #[cfg(target_family = "wasm")]
    {
        use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().without_time())
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("gpui_component=trace".parse().unwrap()),
            )
            .try_init();
    }

    // rust_i18n::extend!(gpui_component);
    gpui_component::init(cx);
    bit7z_pres_components::init(cx);
    // Initialize settings subsystem
    bit7z_pres_settings::init(cx);
    // AppState::init(cx);
    bit7z_pres_theme::theme::init(cx);
    // stories::init(cx);

    // #[cfg(not(target_family = "wasm"))]
    // {
    //     let http_client =
    //         reqwest_client::ReqwestClient::user_agent("gpui-component/story").unwrap();
    //     cx.set_http_client(std::sync::Arc::new(http_client));
    // }
    //
    // #[cfg(target_family = "wasm")]
    // {
    //     // Safety: the web examples run single-threaded; the client is
    //     // created and used exclusively on the main thread.
    //     let http_client = unsafe {
    //         gpui_web::FetchHttpClient::with_user_agent("gpui-component/story")
    //             .expect("failed to create FetchHttpClient")
    //     };
    //     cx.set_http_client(std::sync::Arc::new(http_client));
    // }

    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-f", ToggleSearch, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", ToggleSearch, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", Open, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", Open, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| {
        cx.quit();
    });

    cx.on_action(|_: &About, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            // window.open_alert_dialog(cx, |alert, _, _| {
                            //     alert.title("About").description(markdown(
                            //         "GPUI Component Storybook\n\n\
                            //         Version 0.1.0\n\n\
                            //         https://longbridge.github.io/gpui-component",
                            //     ))
                            // });
                            AboutDialog::open(cx);
                        });
                    })
                    .unwrap();
            });
        }
    });

    cx.on_action(|_: &DebugInfo, cx: &mut App| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, ctx| {
                        window.defer(ctx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, cx| {
                                let startup_path = cx
                                    .app_path()
                                    .unwrap_or(PathBuf::new())
                                    .to_string_lossy()
                                    .to_string();
                                let text_render_mode = cx.text_rendering_mode().clone();
                                alert
                                    .h_96()
                                    .title("Debug Info")
                                    .content(move |content, _window, cx| {
                                        content.child(
                                            div()
                                                .flex_1()
                                                .overflow_y_scrollbar()
                                                .child(
                                                    Text::from(format!(
                                                        "App path: {}\n\
                                                        Text Render Mode: {}\n\
                                                        Theme name: {}\n\
                                                        Compositor: {}\n\
                                                        window_appearance: {:?}\n\
                                                        thermal_state: {:?}\n\
                                                        All Fonts: {:?}\n\
                                                        Displays: {:?}\n\
                                                        primary_display:{:?}\n\
                                                        is_screen_capture_supported: {:?}",
                                                        startup_path,
                                                        render_mode_to_string(text_render_mode),
                                                        cx.theme().theme_name(),
                                                        cx.compositor_name().as_str().to_string(),
                                                        // cx.all_action_names(),
                                                        cx.window_appearance(),
                                                        cx.thermal_state(),
                                                        cx.text_system().all_font_names(),
                                                        cx.displays(),
                                                        cx.primary_display(),
                                                        cx.is_screen_capture_supported()
                                                    ))
                                                )
                                        )
                                    })
                            });
                        })
                    }).unwrap();
            });
        }
    });

    create_new_window_with_size(
        "Bit7z Archiver",
        Some(size(px(800.), px(600.))),
        move |w, cx| {
            let focus_handle = cx.focus_handle();
            w.defer(cx, move |window, cx| {
                if window.focused(cx).is_none() {
                    focus_handle.focus(window, cx);
                }
            });

            let fm_app = ArchiveFileManager::view(None, None, repo,w, cx);

            let app_shell = cx.new(|cx| StoryRoot::new("Bit7z Archiver", fm_app, w, cx));

            app_shell
        },
        cx,
    );

    cx.activate(true);
}
