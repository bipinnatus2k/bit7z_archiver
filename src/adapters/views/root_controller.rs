use crate::application::{
    add_to::AddToArchiveUseCase,
    checksum::{CalculateChecksumUseCase, ChecksumAlgorithm, ChecksumResult},
    create::{CreateArchiveInput, CreateArchiveUseCase},
    delete::DeleteEntriesUseCase,
    extract::ExtractEntriesUseCase,
    open::{OpenArchiveOutput, OpenArchiveUseCase},
    open_entry::OpenEntryUseCase,
    preview::{PreviewData, PreviewEntryUseCase},
    progress::{CrossbeamNotifier, ProgressSender},
    rename::RenameEntryUseCase,
    test::TestEntriesUseCase,
};
use crate::domain::{
    archive::*,
    repository::*,
};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Clone)]
pub struct RootController {
    repo: Arc<dyn ArchiveRepository>,
}

impl RootController {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn repo(&self) -> Arc<dyn ArchiveRepository> {
        self.repo.clone()
    }

    pub fn open_archive(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<OpenArchiveOutput, ArchiveError> {
        let uc = OpenArchiveUseCase::new(self.repo.clone());
        uc.execute(path, password)
    }

    pub fn create_archive(
        &self,
        input: &CreateArchiveInput,
    ) -> Result<ArchiveHandle, ArchiveError> {
        let uc = CreateArchiveUseCase::new(self.repo.clone());
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
        if let Some(tx) = progress {
            self.repo.set_overwrite_mode(overwrite_mode);
            if let Some(ref c) = cancel { self.repo.set_cancel_flag(c.clone()); }
            if let Some(ref p) = paused { self.repo.set_pause_flag(p.clone()); }
            let tx_end = tx.clone();
            let _ = tx.send(crate::application::progress::ProgressUpdate {
                file_current: 0, file_total: expanded.len() as u64,
                current_file: Some(format!("Extracting {} items to {}", expanded.len(), dest.display())),
                items_done: 0, items_total: expanded.len() as u64,
                bytes_done: 0, bytes_total: 100,
                error: None,
            });
            let result = self.repo.extract_with_progress(archive, &expanded, dest, &crate::application::progress::CrossbeamNotifier(tx.clone()));
            let err_str = result.as_ref().err().map(|e| {
                format!("Extract error (id={}, dest={}): {}", archive.id, dest.display(), e)
            });
            if let Some(ref msg) = err_str {
                eprintln!("{}", msg);
            }
            let _ = tx_end.send(crate::application::progress::ProgressUpdate {
                file_current: 0, file_total: 0,
                current_file: None,
                items_done: if result.is_ok() { expanded.len() as u64 } else { 0 },
                items_total: expanded.len() as u64,
                bytes_done: 100, bytes_total: 100,
                error: err_str,
            });
            result
        } else {
            let uc = ExtractEntriesUseCase::new(self.repo.clone());
            uc.execute(archive, &expanded, dest)
        }
    }

    /// Expand selection to include all child items of selected directories.
    pub fn expand_indices(&self, archive: &ArchiveHandle, indices: &[u32]) -> Result<Vec<u32>, ArchiveError> {
        let mut expanded = Vec::new();
        for &idx in indices {
            if let Ok(page) = self.repo.list_page(archive, idx as usize, 1) {
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
        if let Ok(children) = self.repo.list_directory(archive, &path) {
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
        self.repo.extract_to_buffer(archive, index)
    }

    pub fn add_files(
        &self,
        archive: &mut ArchiveHandle,
        files: &[std::path::PathBuf],
        progress: Option<ProgressSender>,
        password: Option<&Password>,
    ) -> Result<(), ArchiveError> {
        let uc = AddToArchiveUseCase::new(self.repo.clone());

        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }

        if let Some(pw) = password {
            uc.execute_with_password(archive, files, None, Some(pw))
        } else {
            uc.execute(archive, files, None)
        }
    }

    pub fn delete_entries(
        &self,
        archive: &mut ArchiveHandle,
        indices: &[u32],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }
        let uc = DeleteEntriesUseCase::new(self.repo.clone());
        uc.execute(archive, indices, None)
    }

    pub fn rename_entry(
        &self,
        archive: &mut ArchiveHandle,
        index: u32,
        new_name: &str,
    ) -> Result<(), ArchiveError> {
        let uc = RenameEntryUseCase::new(self.repo.clone());
        uc.execute(archive, index, new_name)
    }

    pub fn test_entries(
        &self,
        archive: &ArchiveHandle,
        indices: Option<&[u32]>,
        progress: Option<ProgressSender>,
    ) -> Result<TestResult, ArchiveError> {
        if let Some(tx) = progress {
            self.repo.set_progress_notifier(Box::new(CrossbeamNotifier(tx)));
        }
        let uc = TestEntriesUseCase::new(self.repo.clone());
        uc.execute(archive, indices, None)
    }

    pub fn open_entry(
        &self,
        archive: &ArchiveHandle,
        index: u32,
    ) -> Result<(), ArchiveError> {
        let uc = OpenEntryUseCase::new(self.repo.clone());
        uc.execute(archive, index)
    }

    pub fn preview_entry(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        max_bytes: usize,
    ) -> Result<PreviewData, ArchiveError> {
        let uc = PreviewEntryUseCase::new(self.repo.clone());
        uc.execute(archive, index, max_bytes)
    }

    pub fn calculate_checksum(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        algorithms: &[ChecksumAlgorithm],
    ) -> Result<Vec<ChecksumResult>, ArchiveError> {
        let uc = CalculateChecksumUseCase::new(self.repo.clone());
        uc.execute(archive, indices, algorithms)
    }

    pub fn list_page(
        &self,
        archive: &ArchiveHandle,
        offset: usize,
        limit: usize,
    ) -> Result<Page<ArchiveEntry>, ArchiveError> {
        self.repo.list_page(archive, offset, limit)
    }

    pub fn list_directory(
        &self,
        archive: &ArchiveHandle,
        path: &str,
    ) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        self.repo.list_directory(archive, path)
    }

    pub fn get_properties(
        &self,
        archive: &ArchiveHandle,
    ) -> Result<ArchiveProperties, ArchiveError> {
        self.repo.get_properties(archive)
    }

    pub fn close_archive(&self, archive: ArchiveHandle) {
        self.repo.close(&archive);
    }
}


