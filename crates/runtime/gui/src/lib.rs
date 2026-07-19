use bit7z_domain::archive::Password;
use bit7z_pres_settings::{self};
use bit7z_pres_views::archive_file_manager::ArchiveFileManager;
use bit7z_pres_views::utils::window::create_new_window_with_size;
use gpui::*;
use gpui_component_assets::Assets;
use std::path::PathBuf;
use std::sync::Arc;
use bit7z_domain::repository::ArchiveRepository;

pub fn run_gui(repo: Arc<dyn ArchiveRepository>) {
    run_gui_with_path(None, None, repo);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<Password>, repo: Arc<dyn ArchiveRepository>) {
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            bit7z_pres_views::init(cx,repo);
        });
}
