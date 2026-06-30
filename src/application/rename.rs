use crate::application::modify::ModifyArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct RenameEntryUseCase {
    inner: ModifyArchiveUseCase,
}

impl RenameEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { inner: ModifyArchiveUseCase::new(repo) }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        self.inner.rename_entry(archive, index, new_name)
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