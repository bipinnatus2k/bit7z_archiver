use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::Path;
use std::sync::Arc;

pub struct OpenArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

pub struct OpenArchiveOutput {
    pub handle: ArchiveHandle,
    pub first_page: Page<ArchiveEntry>,
    pub properties: ArchiveProperties,
}

impl OpenArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<OpenArchiveOutput, ArchiveError> {
        let handle = self.repo.open(path, password)?;
        let properties = self.repo.get_properties(&handle)?;
        let first_page = self.repo.list_page(&handle, 0usize, 200usize)?;
        Ok(OpenArchiveOutput { handle, first_page, properties })
    }
}
