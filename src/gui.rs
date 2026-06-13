use gpui::*;
use crate::theme::Theme;
use crate::domain::repository::RepoGlobal;
use crate::adapters::bit7z::Library;
use crate::adapters::repository::Bit7zRepository;
use crate::adapters::platform;
use crate::domain::preferences::PreferencesRepository;
use crate::adapters::views::root::RootView;
use std::sync::Arc;

pub fn run_gui() {
    Application::new().run(|cx: &mut App| {
        // Load preferences
        let prefs = crate::adapters::preferences_json::JsonPreferencesRepository::new()
            .load().unwrap_or_default();

        // Find and load the 7-Zip library
        let lib_path = platform::find_7z_library()
            .expect("7-Zip library not found. Install 7-Zip or p7zip.");
        let lib_path_str = lib_path.to_string_lossy();
        let lib = Library::open(&lib_path_str)
            .expect("Failed to load 7-Zip library");

        // Create the repository
        let repo: Arc<dyn crate::domain::repository::ArchiveRepository> =
            Arc::new(Bit7zRepository::new(lib));

        // Set globals
        cx.set_global(prefs);
        cx.set_global(RepoGlobal(repo));
        cx.set_global(Theme::default());

        // Open the main window
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(100.), px(100.)),
                    size(px(1200.), px(800.)),
                ))),

                window_background: WindowBackgroundAppearance::Opaque,
                window_decorations: Some(WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| RootView::new(window, cx)
        );
    });
}



