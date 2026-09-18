use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_app_test::TestEntriesUseCase;
use bit7z_domain::archive::progress::ArchiveError;
use bit7z_domain::archive::{ArchiveHandle, Password, TestResult};
use bit7z_infra_progress::progress_channel;
use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, CloseAction
    , WindowDialogOptions,
};
use gpui::*;
use gpui_component::button::ButtonVariants;
use std::sync::Arc;

pub mod test_results;
pub mod content;

pub struct TestDialog;

impl TestDialog {
    pub fn open_with_entries(
        cx: &mut AsyncApp,
        handle: ArchiveHandle,
        indices: Option<Vec<u32>>,
        service: Arc<ArchiveService>,
    ) {
        let total = indices.as_ref().map_or_else(
            || {
                service
                    .get_properties(&handle)
                    .map(|p| p.files_count as usize)
                    .unwrap_or(0)
            },
            |v| count_expanded(&service, &handle, v),
        );

        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Test Archive".into(),
                width: px(520.),
                height: Some(px(200.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| cx.new(move |_| TestContent::new(total, handle, indices, service)),
        );
    }
}



// ---------------------------------------------------------------------------
// Helpers (from old view.rs)
// ---------------------------------------------------------------------------

fn count_files_recursive(
    service: &Arc<ArchiveService>,
    archive: &ArchiveHandle,
    dir_path: &str,
) -> usize {
    let path = if dir_path.ends_with('/') {
        dir_path.to_string()
    } else {
        format!("{}/", dir_path)
    };
    let mut count = 0;
    if let Ok(children) = service.list_directory(archive, &path) {
        for child in &children {
            if child.is_directory {
                count += count_files_recursive(service, archive, &child.path);
            } else {
                count += 1;
            }
        }
    }
    count
}

fn count_expanded(
    service: &Arc<ArchiveService>,
    archive: &ArchiveHandle,
    indices: &[u32],
) -> usize {
    let mut count = 0;
    for &idx in indices {
        if let Ok(page) = service.list_page(archive, idx as usize, 1) {
            if let Some(entry) = page.items.first() {
                if entry.is_directory {
                    count += count_files_recursive(service, archive, &entry.path);
                    continue;
                }
            }
        }
        count += 1;
    }
    count
}
