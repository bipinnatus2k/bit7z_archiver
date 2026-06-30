use crate::application::plan::{ExecutionPlan, ConflictResolution};
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct ModifyArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl ModifyArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn plan(&self, archive: &ArchiveHandle, change_set: ChangeSet)
        -> Result<ExecutionPlan, ArchiveError>
    {
        self.repo.plan_changes(archive, &change_set)
    }

    pub fn execute(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions)
        -> Result<(), ArchiveError>
    {
        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }
        self.repo.apply_changes(archive, plan, options)
    }

    pub fn execute_with_resolutions(
        &self,
        archive: &ArchiveHandle,
        mut plan: ExecutionPlan,
        resolutions: &[ConflictResolution],
        options: &WriteOptions,
    ) -> Result<(), ArchiveError>
    {
        plan.apply_resolutions(resolutions);
        self.repo.apply_changes(archive, &plan, options)
    }
}
