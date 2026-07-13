use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::*;
use std::path::Path;
use std::path::PathBuf;
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
    ) -> Result<ExtractReport, ArchiveError> {
        let req = ExtractRequest {
            indices: indices.to_vec(),
            dest: dest.to_path_buf(),
            overwrite: options.overwrite_mode,
        };
        let ctx = OpCtx {
            cancel: CancellationToken::new(),
            pause: PauseToken::new(),
            progress: Arc::new(NoopSink),
        };
        self.repo.extract(archive, &req, &ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::*;
    use bit7z_domain::repository::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use bit7z_domain::plan::ExecutionPlan;
    use std::ops::Range;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    struct NoopNotifier;
    impl ProgressNotifier for NoopNotifier {
        fn notify(&self, _: &ProgressUpdate) {}
    }

    fn test_extract_options() -> ExtractOptions {
        ExtractOptions {
            overwrite_mode: OverwriteMode::Overwrite,
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        }
    }

    #[test]
    fn test_extract_success() {
        let repo = MockArchiveRepository::arc_with_count(5);
        let uc = ExtractEntriesUseCase::new(repo);
        let handle = ArchiveHandle::new(0);
        let result = uc.execute(&handle, &[0, 1], Path::new(r"C:\dest"), &test_extract_options());
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_invalid_index_returns_error() {
        struct FailExtract;
        impl ArchiveRepository for FailExtract {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list(&self, _: &ArchiveHandle, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn list_dir(&self, _: &ArchiveHandle, _: &str, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &ExtractRequest, _: &OpCtx) -> Result<ExtractReport, ArchiveError> { Err(ArchiveError::Internal("extract failed".into())) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn plan(&self, _: &ArchiveHandle, _: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn apply(&self, _: &ArchiveHandle, _: &ExecutionPlan, _: &WriteOptions, _: &OpCtx) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle, _: &[u32], _: &OpCtx) -> Result<TestReport, ArchiveError> { Ok(TestReport { all_ok: true, total: 0, failed: vec![] }) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = ExtractEntriesUseCase::new(Arc::new(FailExtract));
        let handle = ArchiveHandle::new(0);
        let result = uc.execute(&handle, &[999], Path::new(r"C:\dest"), &test_extract_options());
        assert!(result.is_err());
    }
}
