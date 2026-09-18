use bit7z_domain::archive::{ArchiveHandle, ChangeSet};
use bit7z_domain::plan::{ConflictResolution, ExecutionPlan};
use bit7z_domain::archive::progress::{NoopNotifier, ProgressNotifier, WriteOptions};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use bit7z_domain::archive::error::ArchiveError;
use crate::runtime_service::ArchiveService;

pub struct ModifyArchiveUseCase {
    service: Arc<ArchiveService>,
}

impl ModifyArchiveUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn plan(
        &self,
        archive: &ArchiveHandle,
        change_set: ChangeSet,
    ) -> Result<ExecutionPlan, ArchiveError> {
        self.service.plan_changes(archive, &change_set)
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        plan: &ExecutionPlan,
        options: &WriteOptions,
    ) -> Result<(), ArchiveError> {
        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }
        self.service.apply_changes(archive, plan, options)
    }

    pub fn execute_with_resolutions(
        &self,
        archive: &ArchiveHandle,
        mut plan: ExecutionPlan,
        resolutions: &[ConflictResolution],
        options: &WriteOptions,
    ) -> Result<(), ArchiveError> {
        plan.apply_resolutions(resolutions);
        self.service.apply_changes(archive, &plan, options)
    }

    pub fn add_files(
        &self,
        archive: &ArchiveHandle,
        files: &[PathBuf],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        for f in files {
            if !f.is_file() && !f.is_dir() {
                return Err(ArchiveError::NotFound(f.to_string_lossy().to_string()));
            }
        }

        let mut change_set = ChangeSet::new();
        for f in files {
            let archive_path = f
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            change_set.add(f.clone(), archive_path);
        }

        let plan = self.service.plan_changes(archive, &change_set)?;

        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }

        let options = WriteOptions {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: progress.unwrap_or_else(|| Arc::new(NoopNotifier)),
        };

        self.service.apply_changes(archive, &plan, &options)
    }

    pub fn delete_entries(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        let mut change_set = ChangeSet::new();
        for &idx in indices {
            change_set.delete(idx);
        }

        let plan = self.service.plan_changes(archive, &change_set)?;

        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }

        let options = WriteOptions {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: progress.unwrap_or_else(|| Arc::new(NoopNotifier)),
        };

        self.service.apply_changes(archive, &plan, &options)
    }

    pub fn rename_entry(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        let mut change_set = ChangeSet::new();
        change_set.rename(index, new_name.to_string());

        let plan = self.service.plan_changes(archive, &change_set)?;

        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict);
        }

        let options = WriteOptions {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        };

        self.service.apply_changes(archive, &plan, &options)
    }
}
