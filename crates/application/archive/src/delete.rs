use crate::modify::ModifyArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::{ArchiveError, ProgressNotifier};
use std::sync::Arc;

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
