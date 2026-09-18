use crate::use_case::modify::ModifyArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::archive::progress::ProgressNotifier;
use std::sync::Arc;
use bit7z_domain::archive::error::ArchiveError;

pub struct DeleteEntriesUseCase {
    inner: ModifyArchiveUseCase,
}

impl DeleteEntriesUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self {
            inner: ModifyArchiveUseCase::new(service),
        }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        self.inner.delete_entries(archive, indices, progress)
    }
}
