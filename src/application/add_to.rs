use crate::application::plan::ExecutionPlan;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

struct NoopNotifier;

impl ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &ProgressUpdate) {}
}

pub struct AddToArchiveUseCase { repo: Arc<dyn ArchiveRepository> }

impl AddToArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        files: &[PathBuf],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        for f in files {
            if !f.is_file() && !f.is_dir() {
                return Err(ArchiveError::NotFound(f.to_string_lossy().to_string()));
            }
        }

        let mut change_set = ChangeSet::new();
        for f in files {
            let archive_path = f.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            change_set.add(f.clone(), archive_path);
        }

        let plan = self.repo.plan_changes(archive, &change_set)?;

        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }

        let options = WriteOptions {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: progress.unwrap_or_else(|| Arc::new(NoopNotifier)),
        };

        self.repo.apply_changes(archive, &plan, &options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_add_to_archive_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let test_file = std::env::temp_dir().join("add_to_test_file.txt");
        std::fs::write(&test_file, b"test").unwrap();
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[test_file.clone()], None);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_add_to_archive_file_not_found() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[PathBuf::from(r"Z:\nonexistent\file.txt")], None);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
