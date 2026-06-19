use crate::application::progress::ProgressSender;
use crate::domain::archive::*;
use crate::domain::repository::*;
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
    ) -> Result<(), ArchiveError> {
        if let Some(tx) = progress {
            self.repo.set_progress_sender(tx);
        }
        self.repo.extract(archive, indices, dest)
    }
}
