use crate::modify::ModifyArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::{ArchiveError, ProgressNotifier};
use std::path::PathBuf;
use std::sync::Arc;

pub struct AddToArchiveUseCase {
    inner: ModifyArchiveUseCase,
}

impl AddToArchiveUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self {
            inner: ModifyArchiveUseCase::new(service),
        }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        files: &[PathBuf],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        self.inner.add_files(archive, files, progress)
    }
}
