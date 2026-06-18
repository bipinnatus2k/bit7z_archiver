use gpui::*;
use gpui_component::*;
use crate::theme::Theme;
use crate::domain::repository::RepoGlobal;
use crate::adapters::bit7z::Library;
use crate::adapters::repository::Bit7zRepository;
use crate::adapters::platform;
use crate::adapters::tray::{TrayManager, TrayGlobal};
use crate::domain::preferences::{PreferencesRepoGlobal, PreferencesRepository, ThemeMode};
use crate::adapters::views::root::RootView;
use std::sync::Arc;

pub fn run_gui() {
    gpui_platform::application().run(|cx: &mut App| {
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
        cx.set_global(RepoGlobal(repo));
        cx.set_global(PreferencesRepoGlobal(Arc::new(prefs_repo)));
        cx.set_global(TrayGlobal(tray));

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.), px(100.)),
                        size(px(1200.), px(800.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                }, |window, cx| {
                    let prefs = cx.global::<crate::domain::preferences::Preferences>();
                    let theme = Theme::from_mode(prefs.ui.theme, window, cx);
                    cx.set_global(theme);

                    let view = RootView::new(window, cx);
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window")
        }).detach();
    });
}



