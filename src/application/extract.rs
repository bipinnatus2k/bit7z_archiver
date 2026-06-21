use crate::domain::archive::ArchiveHandle;
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
    ) -> Result<(), ArchiveError> {
        self.repo.extract(archive, indices, dest)
    }
}
