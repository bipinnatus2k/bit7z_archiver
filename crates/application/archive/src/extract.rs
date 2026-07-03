use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::{ArchiveRepository, ArchiveError, ExtractOptions};
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
        options: &ExtractOptions,
    ) -> Result<(), ArchiveError> {
        self.repo.extract(archive, indices, dest, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::{ArchiveEntry, ArchiveFormat, ArchiveHandle, ChangeSet, EncryptionConfig, Page, Password, TestResult};
    use bit7z_domain::repository::ArchiveProperties;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use bit7z_domain::plan::ExecutionPlan;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    struct NoopNotifier;
    impl bit7z_domain::repository::ProgressNotifier for NoopNotifier {
        fn notify(&self, _: &bit7z_domain::repository::ProgressUpdate) {}
    }

    fn test_extract_options() -> ExtractOptions {
        ExtractOptions {
            overwrite_mode: bit7z_domain::archive::OverwriteMode::Overwrite,
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        }
    }

    #[test]
    fn test_extract_success() {
        let repo = MockArchiveRepository::arc_with_count(5);
        let uc = ExtractEntriesUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[0, 1], Path::new(r"C:\dest"), &test_extract_options());
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_invalid_index_returns_error() {
        struct FailExtract;
        impl ArchiveRepository for FailExtract {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &std::path::Path, _: &ExtractOptions) -> Result<(), ArchiveError> { Err(ArchiveError::Internal("extract failed".into())) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn plan_changes(&self, _: &ArchiveHandle, _: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn apply_changes(&self, _: &ArchiveHandle, _: &ExecutionPlan, _: &bit7z_domain::repository::WriteOptions) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult { total: 0, passed: 0, failed: vec![] }) }
            fn list_directory(&self, _: &ArchiveHandle, _: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> { Ok(vec![]) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = ExtractEntriesUseCase::new(Arc::new(FailExtract));
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[999], Path::new(r"C:\dest"), &test_extract_options());
        assert!(result.is_err());
    }
}
