use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use std::path::Path;
use std::sync::Arc;

pub struct OpenArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

pub struct OpenArchiveOutput {
    pub handle: ArchiveHandle,
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
        let properties = self.repo.properties(&handle)?;
        Ok(OpenArchiveOutput { handle, properties })
    }
}

#[cfg(test)]
mod tests {
    use bit7z_domain::archive::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use bit7z_domain::repository::*;
    use std::ops::Range;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_open_archive_success() {
        let repo = MockArchiveRepository::arc_with_count(3);
        let uc = super::OpenArchiveUseCase::new(repo);
        let result = uc.execute(Path::new("test.7z"), None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.properties.items_count(), 3);
    }

    #[test]
    fn test_open_archive_not_found() {
        struct NotFoundRepo;
        impl ArchiveRepository for NotFoundRepo {
            fn open(&self, p: &Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
                Err(ArchiveError::NotFound(p.to_string_lossy().to_string()))
            }
            fn create(&self, _: &Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list(&self, _: &ArchiveHandle, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn list_dir(&self, _: &ArchiveHandle, _: &str, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &ExtractRequest, _: &OpCtx) -> Result<ExtractReport, ArchiveError> { Ok(ExtractReport::default()) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn plan(&self, _: &ArchiveHandle, _: &ChangeSet) -> Result<bit7z_domain::plan::ExecutionPlan, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn apply(&self, _: &ArchiveHandle, _: &bit7z_domain::plan::ExecutionPlan, _: &WriteOptions, _: &OpCtx) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle, _: &[u32], _: &OpCtx) -> Result<TestReport, ArchiveError> { Ok(TestReport { all_ok: true, total: 0, failed: vec![] }) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = super::OpenArchiveUseCase::new(Arc::new(NotFoundRepo));
        let result = uc.execute(Path::new("missing.7z"), None);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
