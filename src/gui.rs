use gpui::*;
use gpui_component::*;
use gpui_component_assets::Assets;
use crate::theme::Theme;
use crate::domain::repository::RepoGlobal;
use crate::adapters::bit7z::Library;
use crate::adapters::repository::Bit7zRepository;
use crate::adapters::platform;
use crate::adapters::tray::{TrayManager, TrayGlobal};
use crate::domain::preferences::{PreferencesRepoGlobal, PreferencesRepository, ThemeMode};
use crate::adapters::view_models::progress_vm::ProgressState;
use crate::adapters::views::root::RootView;
use std::path::PathBuf;
use std::sync::Arc;

pub fn run_gui() {
    run_gui_with_path(None, None);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<String>) {
    gpui_platform::application().with_assets(Assets).run(move |cx: &mut App| {
        gpui_component::init(cx);
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

        cx.set_global(prefs);
        cx.set_global(RepoGlobal(repo.clone()));
        cx.set_global(PreferencesRepoGlobal(Arc::new(prefs_repo)));
        cx.set_global(TrayGlobal(tray.clone()));
        cx.set_global(ProgressState::default());
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.tray_sender = Some(tray.cmd_tx.clone());
        });

        // Start IPC listener for CLI→GUI handoff
        if let Some(ref open_path) = open_path {
            crate::ipc_connect::start_listener(open_path.as_ref(), {
                let repo = repo.clone();
                move |cmd| {
                    use crate::ipc::GuiCommand;
                    match cmd {
                        GuiCommand::Open { path, password } => {
                            let repo = repo.clone();
                            // Dispatch to the opened window via global
                            // The actual open happens through the ViewModel in RootView
                            log::info!("IPC open: {} (password: {:?})", path, password.is_some());
                        }
                        GuiCommand::Activate => {
                            log::info!("IPC activate");
                        }
                    }
                }
            });
        }

        let open_path = open_path.clone();
        let open_password = open_password.clone();

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Option::from(TitleBar::title_bar_options()),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.), px(100.)),
                        size(px(800.), px(600.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                }, |window, cx| {
                    let prefs = cx.global::<crate::domain::preferences::Preferences>();
                    let theme = Theme::from_mode(prefs.ui.theme, window);
                    cx.set_global(theme);

                    let view = RootView::new(window, cx, open_path.map(|p| p.to_string_lossy().to_string()), open_password);
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window")
        }).detach();
    });
}



