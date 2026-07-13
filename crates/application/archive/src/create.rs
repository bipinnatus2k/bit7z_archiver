use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use std::ops::Range;
use std::sync::Arc;

pub struct CreateArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

pub struct CreateArchiveInput {
    pub destination: std::path::PathBuf,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub encryption: Option<EncryptionConfig>,
}

impl CreateArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        input: &CreateArchiveInput,
    ) -> Result<ArchiveHandle, ArchiveError> {
        self.repo.create(
            &input.destination,
            input.format,
            input.encryption.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::ops::Range;
    use std::sync::Arc;

    #[test]
    fn test_create_archive_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = CreateArchiveUseCase::new(repo);
        let input = CreateArchiveInput {
            destination: "test.7z".into(),
            format: ArchiveFormat::SevenZip,
            compression_level: 5,
            encryption: None,
        };
        let result = uc.execute(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_archive_unsupported_format() {
        struct FailOnCreate;
        impl ArchiveRepository for FailOnCreate {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
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
        let uc = CreateArchiveUseCase::new(Arc::new(FailOnCreate));
        let input = CreateArchiveInput {
            destination: "test.rar".into(),
            format: ArchiveFormat::Rar,
            compression_level: 3,
            encryption: None,
        };
        let result = uc.execute(&input);
        assert!(matches!(result, Err(ArchiveError::UnsupportedOperation)));
    }

    #[test]
    fn test_create_archive_with_encryption_config() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = CreateArchiveUseCase::new(repo);
        let input = CreateArchiveInput {
            destination: "secret.7z".into(),
            format: ArchiveFormat::SevenZip,
            compression_level: 5,
            encryption: Some(EncryptionConfig {
                password: Password::new("mypass"),
                method: EncryptionMethod::Aes256,
                encrypt_filenames: true,
            }),
        };
        let result = uc.execute(&input);
        assert!(result.is_ok());
    }
}
