use crate::application::progress::NoopNotifier;
use crate::domain::archive::{ArchiveHandle, Password, ChangeSet};
use crate::domain::repository::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn new_folder(
    repo: Arc<dyn ArchiveRepository>,
    archive: &ArchiveHandle,
    folder_path: &str,
    _password: Option<&Password>,
) -> Result<(), ArchiveError> {
    let folder_path = folder_path.trim_end_matches('/').trim_end_matches('\\');
    if folder_path.is_empty() {
        return Err(ArchiveError::Internal("folder_path must not be empty".into()));
    }

    let mut temp = std::env::temp_dir();
    let placeholder = ".bit7z_keep";
    temp.push(placeholder);
    std::fs::write(&temp, b"").map_err(ArchiveError::Io)?;

    let archive_inner_path = format!("{}/{}", folder_path, placeholder);

    let mut change_set = ChangeSet::new();
    change_set.add(temp, archive_inner_path);

    let plan = repo.plan_changes(archive, &change_set)?;

    if plan.has_conflicts() {
        return Err(ArchiveError::Conflict);
    }

    let options = WriteOptions {
        cancel: Arc::new(AtomicBool::new(false)),
        paused: Arc::new(AtomicBool::new(false)),
        notifier: Arc::new(NoopNotifier),
    };

    repo.apply_changes(archive, &plan, &options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_new_folder_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &handle, "newdir", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_folder_empty_path() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &handle, "", None);
        assert!(matches!(result, Err(ArchiveError::Internal(ref msg)) if msg.contains("empty")));
    }

    #[test]
    fn test_new_folder_nested() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &handle, "parent/child", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_folder_trim_trailing_slash() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &handle, "mydir/", None);
        assert!(result.is_ok());
    }
}
