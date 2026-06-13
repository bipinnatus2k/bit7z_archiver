use crate::domain::archive::*;
use crate::domain::repository::*;
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

    pub fn execute(&self, input: &CreateArchiveInput) -> Result<ArchiveHandle, ArchiveError> {
        self.repo.create(
            &input.destination,
            input.format,
            input.encryption.as_ref(),
        )
    }
}
