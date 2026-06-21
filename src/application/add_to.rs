use crate::application::progress::{CrossbeamNotifier, ProgressSender};
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AddToArchiveUseCase { repo: Arc<dyn ArchiveRepository> }

impl AddToArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(
        &self,
        archive: &mut ArchiveHandle,
        files: &[PathBuf],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        self.execute_with_password(archive, files, progress, None)
    }

    pub fn execute_with_password(
        &self,
        archive: &mut ArchiveHandle,
        files: &[PathBuf],
        progress: Option<ProgressSender>,
        password: Option<&Password>,
    ) -> Result<(), ArchiveError> {
        // Validate input paths exist
        for f in files {
            if !f.is_file() && !f.is_dir() {
                return Err(ArchiveError::NotFound(
                    f.to_string_lossy().to_string(),
                ));
            }
        }
        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }
        self.repo.add(archive, files, password)
    }
}
