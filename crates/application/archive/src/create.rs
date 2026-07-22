use bit7z_domain::archive::{ArchiveFormat, ArchiveHandle, EncryptionConfig};
use bit7z_domain::repository::ArchiveError;
use std::sync::Arc;

use crate::runtime_service::ArchiveService;

pub struct CreateArchiveUseCase {
    service: Arc<ArchiveService>,
}

pub struct CreateArchiveInput {
    pub destination: std::path::PathBuf,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub encryption: Option<EncryptionConfig>,
}

impl CreateArchiveUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(&self, input: &CreateArchiveInput) -> Result<ArchiveHandle, ArchiveError> {
        self.service
            .create(&input.destination, input.format, input.encryption.as_ref())
    }
}
