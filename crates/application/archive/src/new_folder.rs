use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::{ArchiveHandle, ChangeSet, Password};
use bit7z_domain::repository::{ArchiveError, NoopNotifier, WriteOptions};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn new_folder(
    service: Arc<ArchiveService>,
    archive: &ArchiveHandle,
    folder_path: &str,
    _password: Option<&Password>,
) -> Result<(), ArchiveError> {
    let folder_path = folder_path.trim_end_matches('/').trim_end_matches('\\');
    if folder_path.is_empty() {
        return Err(ArchiveError::Internal(
            "folder_path must not be empty".into(),
        ));
    }

    let mut temp = std::env::temp_dir();
    let placeholder = ".bit7z_keep";
    temp.push(placeholder);
    std::fs::write(&temp, b"").map_err(ArchiveError::Io)?;

    let archive_inner_path = format!("{}/{}", folder_path, placeholder);

    let mut change_set = ChangeSet::new();
    change_set.add(temp, archive_inner_path);

    let plan = service.plan_changes(archive, &change_set)?;

    if plan.has_conflicts() {
        return Err(ArchiveError::Conflict);
    }

    let options = WriteOptions {
        cancel: Arc::new(AtomicBool::new(false)),
        paused: Arc::new(AtomicBool::new(false)),
        notifier: Arc::new(NoopNotifier),
    };

    service.apply_changes(archive, &plan, &options)
}
