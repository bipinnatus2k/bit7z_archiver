use crate::application::progress::ProgressSender;
use crate::domain::archive::{OverwriteMode, ArchiveHandle};
use crate::domain::repository::{ArchiveRepository, ArchiveError};
use std::path::Path;
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
        progress: Option<ProgressSender>,
        overwrite_mode: OverwriteMode,
        keep_broken: bool,
    ) -> Result<(), ArchiveError> {
        self.repo.extract(archive, indices, dest, overwrite_mode, keep_broken, progress)
    }
}
