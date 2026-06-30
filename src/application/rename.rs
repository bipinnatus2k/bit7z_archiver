use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

struct NoopNotifier;
impl ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &ProgressUpdate) {}
}

pub struct RenameEntryUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl RenameEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        let mut change_set = ChangeSet::new();
        change_set.rename(index, new_name.to_string());

        let plan = self.repo.plan_changes(archive, &change_set)?;

        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }

        let options = WriteOptions {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        };

        self.repo.apply_changes(archive, &plan, &options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::archive::ArchiveHandle;
    use crate::domain::repository::test_utils::MockArchiveRepository;

    #[test]
    fn test_rename_success() {
        let repo = MockArchiveRepository::arc_with_count(3);
        let uc = RenameEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, "new_name.txt");
        assert!(result.is_ok());
    }
}
