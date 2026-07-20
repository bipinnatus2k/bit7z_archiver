use bit7z_domain::archive::Password;
use bit7z_domain::repository::ArchiveRepository;
use gpui::*;
use gpui_component_assets::Assets;
use std::path::PathBuf;
use std::sync::Arc;

pub fn run_gui(repo: Arc<dyn ArchiveRepository>) {
    run_gui_with_path(None, None, repo);
}

pub fn run_gui_with_path(open_path: Option<PathBuf>, open_password: Option<Password>, repo: Arc<dyn ArchiveRepository>) {
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            bit7z_pres_views::init_with_path(cx, repo, open_path.clone(), open_password);
        });
}
