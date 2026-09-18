use crate::use_case::modify::ModifyArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use std::sync::Arc;
use bit7z_domain::archive::error::ArchiveError;

pub struct RenameEntryUseCase {
    inner: ModifyArchiveUseCase,
}

impl RenameEntryUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self {
            inner: ModifyArchiveUseCase::new(service),
        }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        self.inner.rename_entry(archive, index, new_name)
    }
}
