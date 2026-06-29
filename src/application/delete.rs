use crate::application::progress::{CrossbeamNotifier, ProgressSender};
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct DeleteEntriesUseCase { repo: Arc<dyn ArchiveRepository> }

impl DeleteEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(
        &self,
        archive: &mut ArchiveHandle<Writer>,
        indices: &[u32],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }
        self.repo.delete(archive, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_delete_entries_success() {
        let mock = MockArchiveRepository::with_count(5);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = DeleteEntriesUseCase::new(repo);
        let mut handle = ArchiveHandle::new_writer();
        let result = uc.execute(&mut handle, &[0, 2], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_entries_with_progress() {
        let mock = MockArchiveRepository::with_count(5);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = DeleteEntriesUseCase::new(repo);
        let (tx, _rx) = crate::application::progress::progress_channel();
        let mut handle = ArchiveHandle::new_writer();
        let result = uc.execute(&mut handle, &[1, 3], Some(tx));
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_nonexistent_indices_is_noop() {
        let mock = MockArchiveRepository::with_count(3);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = DeleteEntriesUseCase::new(repo);
        let mut handle = ArchiveHandle::new_writer();
        let result = uc.execute(&mut handle, &[99, 100], None);
        assert!(result.is_ok());
    }
}
