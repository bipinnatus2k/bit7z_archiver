use bit7z_app_archive::runtime_service::{ArchiveService, build_bit7z_runtime};
use bit7z_infra_bit7z::Library;
use bit7z_infra_platform;
use bit7z_infra_tray::TrayManager;
use bit7z_pres_components::{ext_table, window_dialog};
use bit7z_pres_progress::{ProgressState, init as init_progress};
use bit7z_pres_settings::{self, SettingsStore};
use bit7z_pres_views::root::RootView;
use bit7z_pres_views::utils::window::create_new_window_with_size;
use bit7z_rt_app_state::AppState;
use bit7z_rt_ipc::GuiCommand;
use crossbeam_channel::unbounded;
use gpui::*;
use gpui_component::Theme;
use gpui_component_assets::Assets;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn run_gui() {
    run_gui_with_path(None, None);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<String>) {
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            ext_table::init(cx);
            window_dialog::init(cx);

            // Initialize settings subsystem
            bit7z_pres_settings::init(cx);

            let lib_path = bit7z_infra_platform::find_7z_library()
                .expect("7-Zip library not found. Install 7-Zip or p7zip.");
            let lib_path_str = lib_path.to_string_lossy();
            let lib = Library::open(&lib_path_str).expect("Failed to load 7-Zip library");

            let (runtime, resolver) = build_bit7z_runtime(lib);
            let service: Arc<ArchiveService> = Arc::new(ArchiveService::new(runtime, resolver));

            let tray = Arc::new(TrayManager::new());

            // Create IPC channel for CLI→GUI communication
            let (ipc_tx, ipc_rx) = unbounded::<GuiCommand>();

            // Create AppState — single aggregate of all core services
            let app_state = Arc::new(AppState::new(
                service.clone(),
                Arc::new(
                    bit7z_infra_persistence::preferences_json::JsonPreferencesRepository::new(),
                ),
                tray.clone(),
                Arc::new(Mutex::new(ipc_rx)),
            ));
            AppState::set_global(app_state.clone(), cx);

            // Initialize progress state with tray sender
            init_progress(cx, tray.cmd_tx.clone());

            if let Some(ref open_path) = open_path {
                let ipc_tx = ipc_tx.clone();
                bit7z_rt_ipc::start_listener(open_path.as_ref(), move |cmd| {
                    let _ = ipc_tx.send(cmd);
                });
            }

            let open_path2 = open_path.map(|p| p.to_string_lossy().to_string());
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

                    // Apply persisted theme
                    let prefs = &SettingsStore::get(cx).prefs;
                    let theme_mode = match prefs.ui.theme {
                        bit7z_domain::preferences::ThemeMode::Light => {
                            gpui_component::theme::ThemeMode::Light
                        }
                        bit7z_domain::preferences::ThemeMode::Dark => {
                            gpui_component::theme::ThemeMode::Dark
                        }
                        bit7z_domain::preferences::ThemeMode::System => {
                            if w.appearance() == gpui::WindowAppearance::Dark {
                                gpui_component::theme::ThemeMode::Dark
                            } else {
                                gpui_component::theme::ThemeMode::Light
                            }
                        }
                    };
                    Theme::change(theme_mode, Some(w), cx);
                    cx.bind_keys([]);
                    RootView::view(w, cx, open_path2.clone(), open_password)
                },
                cx,
            );
        });
}
