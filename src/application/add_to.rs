use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AddToArchiveUseCase { repo: Arc<dyn ArchiveRepository> }

impl AddToArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(&self, archive: &mut ArchiveHandle, files: &[PathBuf]) -> Result<(), ArchiveError> {
        self.repo.add(archive, files)
    }
}
