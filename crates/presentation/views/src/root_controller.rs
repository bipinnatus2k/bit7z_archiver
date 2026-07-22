use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_app_archive::{
    add_to::AddToArchiveUseCase,
    create::{CreateArchiveInput, CreateArchiveUseCase},
    delete::DeleteEntriesUseCase,
    extract::ExtractEntriesUseCase,
    open::{OpenArchiveOutput, OpenArchiveUseCase},
    open_entry::OpenEntryUseCase,
    rename::RenameEntryUseCase,
};
use bit7z_app_checksum::{CalculateChecksumUseCase, ChecksumAlgorithm, ChecksumResult};
use bit7z_app_preview::{PreviewData, PreviewEntryUseCase};
use bit7z_app_test::TestEntriesUseCase;
use bit7z_domain::{
    archive::*,
    repository::{
        ArchiveError, ArchiveProperties, ExtractOptions, NoopNotifier, ProgressNotifier,
        ProgressUpdate,
    },
};
use bit7z_infra_progress::{CrossbeamNotifier, ProgressSender};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Clone)]
pub struct RootController {
    service: Arc<ArchiveService>,
}

impl RootController {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn service(&self) -> Arc<ArchiveService> {
        self.service.clone()
    }

    pub fn open_archive(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<OpenArchiveOutput, ArchiveError> {
        let uc = OpenArchiveUseCase::new(self.service.clone());
        uc.execute(path, password)
    }

    pub fn create_archive(
        &self,
        input: &CreateArchiveInput,
    ) -> Result<ArchiveHandle, ArchiveError> {
        let uc = CreateArchiveUseCase::new(self.service.clone());
        uc.execute(input)
    }

    pub fn extract(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        overwrite_mode: OverwriteMode,
        progress: Option<ProgressSender>,
        cancel: Option<Arc<AtomicBool>>,
        paused: Option<Arc<AtomicBool>>,
    ) -> Result<(), ArchiveError> {
        let expanded = self.expand_indices(archive, indices)?;

        let (notifier, progress_tx): (Arc<dyn ProgressNotifier>, Option<ProgressSender>) =
            if let Some(tx) = progress {
                let tx_start = tx.clone();
                let _ = tx_start.send(ProgressUpdate {
                    file_current: 0,
                    file_total: expanded.len() as u64,
                    current_file: Some(format!(
                        "Extracting {} items to {}",
                        expanded.len(),
                        dest.display()
                    )),
                    items_done: 0,
                    items_total: expanded.len() as u64,
                    bytes_done: 0,
                    bytes_total: 100,
                    error: None,
                });
                (Arc::new(CrossbeamNotifier(tx.clone())), Some(tx))
            } else {
                (Arc::new(NoopNotifier), None)
            };

        let options = ExtractOptions {
            overwrite_mode,
            cancel: cancel.unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
            paused: paused.unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
            notifier,
        };

        let uc = ExtractEntriesUseCase::new(self.service.clone());
        let result = uc.execute(archive, &expanded, dest, &options);

        if let Some(ref tx) = progress_tx {
            let err_str = result.as_ref().err().map(|e| {
                format!(
                    "Extract error (id={}, dest={}): {}",
                    archive.id,
                    dest.display(),
                    e
                )
            });
            let _ = tx.send(ProgressUpdate {
                file_current: 0,
                file_total: 0,
                current_file: None,
                items_done: if result.is_ok() {
                    expanded.len() as u64
                } else {
                    0
                },
                items_total: expanded.len() as u64,
                bytes_done: 100,
                bytes_total: 100,
                error: err_str,
            });
        }

        result
    }

    pub fn expand_indices(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
    ) -> Result<Vec<u32>, ArchiveError> {
        let mut expanded = Vec::new();
        for &idx in indices {
            if let Ok(page) = self.service.list_page(archive, idx as usize, 1) {
                if let Some(entry) = page.items.first() {
                    if entry.is_directory {
                        self.collect_directory(archive, &entry.path, &mut expanded)?;
                        continue;
                    }
                }
            }
            expanded.push(idx);
        }
        Ok(expanded)
    }

    fn collect_directory(
        &self,
        archive: &ArchiveHandle,
        dir_path: &str,
        expanded: &mut Vec<u32>,
    ) -> Result<(), ArchiveError> {
        let path = if dir_path.ends_with('/') {
            dir_path.to_string()
        } else {
            format!("{}/", dir_path)
        };
        if let Ok(children) = self.service.list_directory(archive, &path) {
            for child in &children {
                if child.is_directory {
                    self.collect_directory(archive, &child.path, expanded)?;
                } else {
                    expanded.push(child.original_index);
                }
            }
        }
        Ok(())
    }

    pub fn extract_to_buffer(
        &self,
        archive: &ArchiveHandle,
        index: u32,
    ) -> Result<Vec<u8>, ArchiveError> {
        self.service.extract_to_buffer(archive, index)
    }

    pub fn add_files(
        &self,
        archive: &ArchiveHandle,
        files: &[std::path::PathBuf],
        progress: Option<ProgressSender>,
        _password: Option<&Password>,
    ) -> Result<(), ArchiveError> {
        let notifier: Option<Arc<dyn ProgressNotifier>> =
            progress.map(|tx| Arc::new(CrossbeamNotifier(tx)) as Arc<dyn ProgressNotifier>);
        let uc = AddToArchiveUseCase::new(self.service.clone());
        uc.execute(archive, files, notifier)
    }

    pub fn delete_entries(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        let notifier: Option<Arc<dyn ProgressNotifier>> =
            progress.map(|tx| Arc::new(CrossbeamNotifier(tx)) as Arc<dyn ProgressNotifier>);
        let uc = DeleteEntriesUseCase::new(self.service.clone());
        uc.execute(archive, indices, notifier)
    }

    pub fn rename_entry(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        let uc = RenameEntryUseCase::new(self.service.clone());
        uc.execute(archive, index, new_name)
    }

    pub fn test_entries(
        &self,
        archive: &ArchiveHandle,
        indices: Option<&[u32]>,
        _progress: Option<ProgressSender>,
    ) -> Result<TestResult, ArchiveError> {
        let uc = TestEntriesUseCase::new(self.service.clone());
        uc.execute(archive, indices, None)
    }

    pub fn open_entry(&self, archive: &ArchiveHandle, index: u32) -> Result<(), ArchiveError> {
        let uc = OpenEntryUseCase::new(self.service.clone());
        uc.execute(archive, index)
    }

    pub fn preview_entry(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        max_bytes: usize,
    ) -> Result<PreviewData, ArchiveError> {
        let uc = PreviewEntryUseCase::new(self.service.clone());
        uc.execute(archive, index, max_bytes)
    }

    pub fn calculate_checksum(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        algorithms: &[ChecksumAlgorithm],
    ) -> Result<Vec<ChecksumResult>, ArchiveError> {
        let uc = CalculateChecksumUseCase::new(self.service.clone());
        uc.execute(archive, indices, algorithms)
    }

    pub fn list_page(
        &self,
        archive: &ArchiveHandle,
        offset: usize,
        limit: usize,
    ) -> Result<Page<ArchiveEntry>, ArchiveError> {
        self.service.list_page(archive, offset, limit)
    }

    pub fn list_directory(
        &self,
        archive: &ArchiveHandle,
        path: &str,
    ) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        self.service.list_directory(archive, path)
    }

    pub fn get_properties(
        &self,
        archive: &ArchiveHandle,
    ) -> Result<ArchiveProperties, ArchiveError> {
        self.service.get_properties(archive)
    }

    pub fn close_archive(&self, archive: ArchiveHandle) {
        self.service.close(&archive);
    }

    pub fn commit_archive(&self, _archive: &ArchiveHandle) -> Result<(), ArchiveError> {
        // The runtime service applies changes immediately via apply_changes.
        // A separate commit step is not required in the new model.
        Ok(())
    }

    pub fn undo_archive(&self, _archive: &ArchiveHandle) -> Result<bool, ArchiveError> {
        // Undo is not supported by the runtime service yet.
        Ok(false)
    }

    pub fn redo_archive(&self, _archive: &ArchiveHandle) -> Result<bool, ArchiveError> {
        // Redo is not supported by the runtime service yet.
        Ok(false)
    }

    pub fn has_unsaved_changes(&self, _archive: &ArchiveHandle) -> bool {
        // The runtime service applies changes immediately; no pending state.
        false
    }

    pub fn can_undo(&self, _archive: &ArchiveHandle) -> bool {
        false
    }

    pub fn can_redo(&self, _archive: &ArchiveHandle) -> bool {
        false
    }

    pub fn discard_pending(&self, _archive: &ArchiveHandle) -> Result<(), ArchiveError> {
        Ok(())
    }
}
