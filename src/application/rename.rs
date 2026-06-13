use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct RenameEntryUseCase { repo: Arc<dyn ArchiveRepository> }

impl RenameEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError> {
        self.repo.rename(archive, index, new_name)
    }
}
