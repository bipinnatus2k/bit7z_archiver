use crate::application::progress::{CrossbeamNotifier, ProgressSender};
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct DeleteEntriesUseCase { repo: Arc<dyn ArchiveRepository> }

impl DeleteEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }
    pub fn execute(
        &self,
        archive: &mut ArchiveHandle,
        indices: &[u32],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }
        self.repo.delete(archive, indices)
    }
}
