use gpui::*;
use gpui_component_assets::Assets;
use bit7z_pres_theme::Theme;
use bit7z_domain::preferences::PreferencesRepository;
use bit7z_infra_bit7z::Library;
use bit7z_infra_persistence::Bit7zRepository;
use bit7z_infra_platform;
use bit7z_infra_tray::{TrayManager, TrayGlobal};
use bit7z_pres_view_models::progress_vm::ProgressState;
use bit7z_pres_views::root::RootView;
use bit7z_pres_views::utils::window::create_new_window_with_size;
use bit7z_pres_components::ext_table;
use bit7z_rt_ipc::GuiCommand;
use bit7z_rt_globals::{IpcReceiver, PreferencesGlobal, RepoGlobal, PreferencesRepoGlobal};
use crossbeam::channel::unbounded;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn run_gui() {
    run_gui_with_path(None, None);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<String>) {
    gpui_platform::application().with_assets(Assets).run(move |cx: &mut App| {
        gpui_component::init(cx);
        ext_table::init(cx);
        let prefs_repo = bit7z_infra_persistence::preferences_json::JsonPreferencesRepository::new();
        let prefs = prefs_repo.load().unwrap_or_default();

        let lib_path = bit7z_infra_platform::find_7z_library()
            .expect("7-Zip library not found. Install 7-Zip or p7zip.");
        let lib_path_str = lib_path.to_string_lossy();
        let lib = Library::open(&lib_path_str)
            .expect("Failed to load 7-Zip library");

        let repo: Arc<dyn bit7z_domain::repository::ArchiveRepository> =
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
            bit7z_rt_ipc::start_listener(open_path.as_ref(), move |cmd| {
                let _ = ipc_tx.send(cmd);
            });
        }

        let open_path2 = open_path.map(|p| {p.to_string_lossy().to_string()});
        create_new_window_with_size(
            "Bit7z Archiver",
            Some(size(px(800.), px(600.))),
            move |w,cx| {

                let focus_handle = cx.focus_handle();
                w.defer(cx, move |window, cx| {
                    if window.focused(cx).is_none() {
                        focus_handle.focus(window, cx);
                    }
                });

                let prefs = &cx.global::<PreferencesGlobal>().0;
                let theme = Theme::from_mode(prefs.ui.theme, w);
                cx.set_global(theme);
                cx.bind_keys([]);
                RootView::view(w, cx, open_path2.clone(), open_password)
            },
            cx,
        );
    });
}
