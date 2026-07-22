use crate::modify::ModifyArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::ArchiveError;
use std::sync::Arc;

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
