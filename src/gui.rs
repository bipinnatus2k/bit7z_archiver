use gpui::*;
use gpui_component::*;
use crate::theme::Theme;
use crate::domain::repository::RepoGlobal;
use crate::adapters::bit7z::Library;
use crate::adapters::repository::Bit7zRepository;
use crate::adapters::platform;
use crate::domain::preferences::PreferencesRepository;
use crate::adapters::views::root::RootView;
use std::sync::Arc;

pub fn run_gui() {
    gpui_platform::application().run(|cx: &mut App| {
        // 使用任何 GPUI Component 功能之前必须先调用此函数。
        gpui_component::init(cx);
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

        cx.spawn(async move |cx| {
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
                }, |window, cx| {
                    let view =  RootView::new(window,cx);
                    // 窗口的第一层应该是一个 Root。
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window")
        }).detach();
    });
}



