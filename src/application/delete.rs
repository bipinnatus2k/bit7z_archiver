use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

struct NoopNotifier;
impl ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &ProgressUpdate) {}
}

pub struct DeleteEntriesUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl DeleteEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        let mut change_set = ChangeSet::new();
        for &idx in indices {
            change_set.delete(idx);
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
    use crate::domain::archive::{ArchiveHandle};
    use crate::domain::repository::test_utils::MockArchiveRepository;

    #[test]
    fn test_delete_success() {
        let repo = MockArchiveRepository::arc_with_count(5);
        let uc = DeleteEntriesUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[0, 1], None);
        assert!(result.is_ok());
    }
}
