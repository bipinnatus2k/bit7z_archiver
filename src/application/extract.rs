use crate::domain::archive::ArchiveHandle;
use crate::domain::repository::{ArchiveRepository, ArchiveError};
use std::path::Path;
use std::sync::Arc;

pub struct ExtractEntriesUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl ExtractEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
    ) -> Result<(), ArchiveError> {
        self.repo.extract(archive, indices, dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::archive::{ArchiveEntry, ArchiveFormat, ArchiveHandle, EncryptionConfig, Page, Password, TestResult, Writer};
    use crate::domain::repository::ArchiveProperties;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_extract_success() {
        let repo = MockArchiveRepository::arc_with_count(5);
        let uc = ExtractEntriesUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[0, 1], Path::new(r"C:\dest"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_invalid_index_returns_error() {
        struct FailExtract;
        impl ArchiveRepository for FailExtract {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle<Writer>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &std::path::Path) -> Result<(), ArchiveError> { Err(ArchiveError::Internal("extract failed".into())) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn add(&self, _: &mut ArchiveHandle<Writer>, _: &[std::path::PathBuf], _: Option<&Password>) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn delete(&self, _: &mut ArchiveHandle<Writer>, _: &[u32]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn rename(&self, _: &mut ArchiveHandle<Writer>, _: u32, _: &str) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult { total: 0, passed: 0, failed: vec![] }) }
            fn list_directory(&self, _: &ArchiveHandle, _: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> { Ok(vec![]) }
            fn close(&self, _: &ArchiveHandle) {}
            fn close_writer(&self, _: &ArchiveHandle<Writer>) {}
        }
        let uc = ExtractEntriesUseCase::new(Arc::new(FailExtract));
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[999], Path::new(r"C:\dest"));
        assert!(result.is_err());
    }
}
