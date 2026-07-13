use gpui::*;
use gpui_component::Theme;
use gpui_component_assets::Assets;
use bit7z_infra_platform;
use bit7z_infra_tray::TrayManager;
use bit7z_pres_settings::{self, SettingsStore};
use bit7z_pres_progress::init as init_progress;
use bit7z_pres_views::utils::window::create_new_window_with_size;
use bit7z_pres_components::{ext_table, window_dialog};
use bit7z_rt_app_state::AppState;
use bit7z_rt_ipc::GuiCommand;
use crossbeam_channel::unbounded;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn run_gui() {
    run_gui_with_path(None, None);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<String>) {
    gpui_platform::application().with_assets(Assets).run(move |cx: &mut App| {
        gpui_component::init(cx);
        ext_table::init(cx);
        window_dialog::init(cx);

        // Initialize settings subsystem
        bit7z_pres_settings::init(cx);

        let lib_path = bit7z_infra_platform::find_7z_library()
            .expect("7-Zip library not found. Install 7-Zip or p7zip.");
        let lib_path_str = lib_path.to_string_lossy();

        let repo: Arc<dyn bit7z_domain::repository::ArchiveRepository> =
            Arc::new(bit7z_infra_repo::supervisor::RepoSupervisor::new(&lib_path_str)
                .expect("Failed to create RepoSupervisor"));

        let tray = Arc::new(TrayManager::new());

        // Create IPC channel for CLI→GUI communication
        let (ipc_tx, ipc_rx) = unbounded::<GuiCommand>();

        // Create AppState — single aggregate of all core services
        let app_state = Arc::new(AppState::new(
            repo.clone(),
            Arc::new(bit7z_infra_persistence::preferences_json::JsonPreferencesRepository::new()),
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
                    bit7z_domain::preferences::ThemeMode::Light => gpui_component::theme::ThemeMode::Light,
                    bit7z_domain::preferences::ThemeMode::Dark => gpui_component::theme::ThemeMode::Dark,
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
                let app_shell = cx.new(|cx| bit7z_pres_views::app_shell::AppShell::new(w, cx, repo, open_path2.clone(), open_password));

                // IPC polling: spawn from the AppShell entity context
                let ipc_rx_clone = app_state.ipc_receiver.clone();
                app_shell.update(cx, |_shell, cx| {
                    let weak = cx.entity().downgrade();
                    cx.spawn(async move |this, cx| {
                        loop {
                            while let Ok(cmd) = ipc_rx_clone.lock().unwrap().try_recv() {
                                match cmd {
                                    GuiCommand::Open { path, password } => {
                                        let _ = this.update(cx, |shell, cx| {
                                            shell.handle_open_archive(
                                                std::path::Path::new(&path),
                                                password,
                                                cx,
                                            );
                                        });
                                    }
                                    GuiCommand::Activate => {
                                        // TODO: bring window to front
                                    }
                                }
                            }
                            smol::Timer::after(Duration::from_millis(50)).await;
                        }
                    }).detach();
                });

                app_shell.update(cx, |shell, _| shell.root_view.clone())
            },
            cx,
        );
    });
}
