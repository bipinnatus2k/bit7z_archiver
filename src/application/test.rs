use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct TestArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl TestArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        self.repo.test(archive)
    }
}
