use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::Path;
use std::sync::Arc;

pub struct OpenArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

pub struct OpenArchiveOutput {
    pub handle: ArchiveHandle,
    pub first_page: Page<ArchiveEntry>,
    pub properties: ArchiveProperties,
}

impl OpenArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<OpenArchiveOutput, ArchiveError> {
        let handle = self.repo.open(path, password)?;
        let properties = self.repo.get_properties(&handle)?;
        let first_page = self.repo.list_page(&handle, 0usize, 200usize)?;
        Ok(OpenArchiveOutput { handle, first_page, properties })
    }
}

/// Use case tests with MockArchiveRepository.
#[cfg(test)]
mod tests {
    use crate::domain::archive::*;
    use crate::domain::repository::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_open_archive_success() {
        let repo = MockArchiveRepository::arc_with_count(3);
        let uc = crate::application::open::OpenArchiveUseCase::new(repo);
        let result = uc.execute(Path::new("test.7z"), None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.first_page.items.len(), 3);
        assert_eq!(output.properties.items_count, 3);
    }

    #[test]
    fn test_open_archive_not_found() {
        struct NotFoundRepo;
        impl ArchiveRepository for NotFoundRepo {
            fn open(&self, p: &Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
                Err(ArchiveError::NotFound(p.to_string_lossy().to_string()))
            }
            fn create(&self, _: &Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties {
                items_count:0, folders_count:0, files_count:0, total_size:0, packed_size:0,
                is_encrypted:false, has_encrypted_items:false, is_multi_volume:false, is_solid:false,
            }) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &Path) -> Result<(), ArchiveError> { Ok(()) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn add(&self, _: &mut ArchiveHandle, _: &[std::path::PathBuf]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn delete(&self, _: &mut ArchiveHandle, _: &[u32]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn rename(&self, _: &mut ArchiveHandle, _: u32, _: &str) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult{total:0, passed:0, failures:vec![]}) }
            fn close(&self, _: ArchiveHandle) {}
        }
        let uc = crate::application::open::OpenArchiveUseCase::new(Arc::new(NotFoundRepo));
        let result = uc.execute(Path::new("missing.7z"), None);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
