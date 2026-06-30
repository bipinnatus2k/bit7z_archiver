use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct RenameEntryUseCase { repo: Arc<dyn ArchiveRepository> }

impl RenameEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(&self, archive: &ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError> {
        self.repo.rename(archive, index, new_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_rename_entry_success() {
        let repo = MockArchiveRepository::arc_with_count(5);
        let uc = RenameEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, "renamed.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rename_nonexistent_entry_returns_error() {
        struct FailRename;
        impl ArchiveRepository for FailRename {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &std::path::Path) -> Result<(), ArchiveError> { Ok(()) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn add(&self, _: &ArchiveHandle, _: &[std::path::PathBuf], _: Option<&Password>) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn delete(&self, _: &ArchiveHandle, _: &[u32]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn rename(&self, _: &ArchiveHandle, _: u32, _: &str) -> Result<(), ArchiveError> { Err(ArchiveError::NotFound("entry not found".into())) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult { total: 0, passed: 0, failed: vec![] }) }
            fn list_directory(&self, _: &ArchiveHandle, _: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> { Ok(vec![]) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = RenameEntryUseCase::new(Arc::new(FailRename));
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 99, "gone.txt");
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
