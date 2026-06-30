use crate::application::modify::ModifyArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct DeleteEntriesUseCase {
    inner: ModifyArchiveUseCase,
}

impl DeleteEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { inner: ModifyArchiveUseCase::new(repo) }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        self.inner.delete_entries(archive, indices, progress)
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