use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;
use crate::theme::Theme;
use crate::domain::repository::ArchiveRepository;
use crate::adapters::bit7z::Library;
use crate::adapters::repository::Bit7zRepository;
use crate::adapters::platform;
use crate::adapters::tray::{TrayManager, TrayGlobal};
use crate::domain::preferences::{Preferences, PreferencesRepository};
use crate::adapters::view_models::progress_vm::ProgressState;
use crate::adapters::views::root::RootView;
use crate::ipc::GuiCommand;
use crossbeam::channel::unbounded;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::adapters::views::utils::window::create_new_window_with_size;

/// Global receiver for IPC commands from CLI.
pub struct IpcReceiver(pub Arc<Mutex<crossbeam::channel::Receiver<GuiCommand>>>);
impl Global for IpcReceiver {}

/// Global wrapper for Preferences (avoids gpui::Global in domain).
pub struct PreferencesGlobal(pub Preferences);
impl Global for PreferencesGlobal {}

/// Global wrapper for ArchiveRepository (avoids gpui::Global in domain).
pub struct RepoGlobal(pub Arc<dyn ArchiveRepository>);
impl Global for RepoGlobal {}

/// Global wrapper for PreferencesRepository (avoids gpui::Global in domain).
pub struct PreferencesRepoGlobal(pub Arc<dyn crate::domain::preferences::PreferencesRepository>);
impl Global for PreferencesRepoGlobal {}



pub fn run_gui() {
    run_gui_with_path(None, None);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<String>) {
    gpui_platform::application().with_assets(Assets).run(move |cx: &mut App| {
        gpui_component::init(cx);
        crate::adapters::views::ext_table::init(cx);
        let prefs_repo = crate::adapters::preferences_json::JsonPreferencesRepository::new();
        let prefs = prefs_repo.load().unwrap_or_default();

        let lib_path = platform::find_7z_library()
            .expect("7-Zip library not found. Install 7-Zip or p7zip.");
        let lib_path_str = lib_path.to_string_lossy();
        let lib = Library::open(&lib_path_str)
            .expect("Failed to load 7-Zip library");

        let repo: Arc<dyn crate::domain::repository::ArchiveRepository> =
            Arc::new(Bit7zRepository::new(lib));

        let tray = Arc::new(TrayManager::new());

        // Create IPC channel for CLI→GUI communication
        let (ipc_tx, ipc_rx) = unbounded::<GuiCommand>();
        cx.set_global(IpcReceiver(Arc::new(Mutex::new(ipc_rx))));

        cx.set_global(PreferencesGlobal(prefs));
        cx.set_global(RepoGlobal(repo.clone()));
        cx.set_global(PreferencesRepoGlobal(Arc::new(prefs_repo)));
        cx.set_global(TrayGlobal(tray.clone()));
        cx.set_global(ProgressState::default());
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.tray_sender = Some(tray.cmd_tx.clone());
        });

        // Start IPC listener for CLI→GUI handoff
        if let Some(ref open_path) = open_path {
            let ipc_tx = ipc_tx.clone();
            crate::ipc_connect::start_listener(open_path.as_ref(), move |cmd| {
                let _ = ipc_tx.send(cmd);
            });
        }

        let open_path2 = open_path.map(|p| {p.to_string_lossy().to_string()});
        create_new_window_with_size(
            "Bit7z Archiver",
            Some(size(px(800.), px(600.))),
            move |w,cx| {

                // Set focus to the StoryRoot to enable it's actions.
                let focus_handle = cx.focus_handle();
                w.defer(cx, move |window, cx| {
                    if window.focused(cx).is_none() {
                        focus_handle.focus(window, cx);
                    }
                });
                
                let prefs = &cx.global::<PreferencesGlobal>().0;
                let theme = Theme::from_mode(prefs.ui.theme, w);
                cx.set_global(theme);
                cx.bind_keys([
                    // KeyBinding::new()
                ]);
                RootView::view(w, cx, open_path2.clone(), open_password)
            },
            cx,
        );

        // cx.spawn(async move |cx| {
        //     let bounds = cx.update(|app| {
        //         WindowBounds::centered(size(px(800.), px(600.)), app)
        //     });
        //
        //
        //     cx.open_window(
        //         WindowOptions {
        //             titlebar: Option::from(TitleBar::title_bar_options()),
        //             window_bounds: Some(bounds),
        //             window_background: WindowBackgroundAppearance::Opaque,
        //             window_decorations: Some(WindowDecorations::Client),
        //             ..Default::default()
        //         }, |window, cx| {
        //             let prefs = &cx.global::<PreferencesGlobal>().0;
        //             let theme = Theme::from_mode(prefs.ui.theme, window);
        //             cx.set_global(theme);
        //
        //             cx.bind_keys(
        //                 [
        //
        //                 ]
        //             );
        //
        //
        //
        //
        //             let view = RootView::new(window, cx, open_path.map(|p| p.to_string_lossy().to_string()), open_password);
        //             // cx.new(|cx| Root::new(view, window, cx))
        //             create_new_window_with_size(
        //                 "Archive File Manager",
        //                 Some(size(px(800.), px(600.))),
        //                 |window, cx| cx.new(|cx| Example::new(window, cx)),,
        //                 cx,
        //                 view
        //             );
        //         })
        //         .expect("Failed to open window")
        // }).detach();
    });
}



