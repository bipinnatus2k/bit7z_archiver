use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::error::ArchiveError;
use bit7z_domain::archive::progress::ExtractOptions;
use bit7z_domain::archive::ArchiveHandle;
use std::path::Path;
use std::sync::Arc;

pub struct ExtractEntriesUseCase {
    service: Arc<ArchiveService>,
}

impl ExtractEntriesUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        options: &ExtractOptions,
    ) -> Result<(), ArchiveError> {
        self.service.extract(archive, indices, dest, options)
    }
}
