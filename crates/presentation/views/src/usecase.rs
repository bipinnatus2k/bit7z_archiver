use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use bit7z_domain::archive::{ArchiveEntry, ArchiveHandle, OverwriteMode};
use bit7z_domain::repository::{ArchiveError, ArchiveProperties, ArchiveRepository, ExtractOptions, NoopNotifier};

pub struct UseCases {
    pub repo: Arc<dyn ArchiveRepository>,
    extract_uc: bit7z_app_archive::extract::ExtractEntriesUseCase,
    delete_uc: bit7z_app_archive::delete::DeleteEntriesUseCase,
    test_uc: bit7z_app_test::TestArchiveUseCase,
    test_entries_uc: bit7z_app_test::TestEntriesUseCase,
    open_entry_uc: bit7z_app_archive::open_entry::OpenEntryUseCase,
    preview_uc: bit7z_app_preview::PreviewEntryUseCase,
}

impl UseCases {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self {
            extract_uc: bit7z_app_archive::extract::ExtractEntriesUseCase::new(repo.clone()),
            delete_uc: bit7z_app_archive::delete::DeleteEntriesUseCase::new(repo.clone()),
            test_uc: bit7z_app_test::TestArchiveUseCase::new(repo.clone()),
            test_entries_uc: bit7z_app_test::TestEntriesUseCase::new(repo.clone()),
            open_entry_uc: bit7z_app_archive::open_entry::OpenEntryUseCase::new(repo.clone()),
            preview_uc: bit7z_app_preview::PreviewEntryUseCase::new(repo.clone()),
            repo,
        }
    }

    pub fn extract(&self, h: &ArchiveHandle, idx: &[u32], dest: &Path, om: OverwriteMode) -> Result<(), ArchiveError> {
        let options = ExtractOptions {
            overwrite_mode: om,
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        };
        self.extract_uc.execute(h, idx, dest, &options).map(|_| ())
    }

    pub fn delete(&self, h: &ArchiveHandle, idx: &[u32]) -> Result<(), ArchiveError> {
        self.delete_uc.execute(h, idx, None)
    }

    pub fn test(&self, h: &ArchiveHandle) -> Result<bit7z_domain::archive::TestResult, ArchiveError> {
        self.test_uc.execute(h)
    }

    pub fn test_selected(&self, h: &ArchiveHandle, idx: &[u32]) -> Result<bit7z_domain::archive::TestResult, ArchiveError> {
        self.test_entries_uc.execute(h, Some(idx), None)
    }

    pub fn open_entry(&self, h: &ArchiveHandle, idx: u32) -> Result<(), ArchiveError> {
        self.open_entry_uc.execute(h, idx)
    }

    pub fn properties(&self, h: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { self.repo.properties(h) }
    pub fn close(&self, h: &ArchiveHandle) { self.repo.close(h); }

    pub fn list_directory(&self, h: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        self.repo.list_dir(h, path, 0..usize::MAX).map(|p| p.items)
    }

    pub fn preview(&self, h: &ArchiveHandle, idx: u32, max: usize) -> Result<bit7z_app_preview::PreviewData, ArchiveError> {
        self.preview_uc.execute(h, idx, max)
    }

    pub fn new_file(&self, h: &ArchiveHandle) -> Result<(), ArchiveError> {
        let _ = bit7z_app_archive::new_file::new_file_and_add(self.repo.clone(), h, "new_file.txt", None);
        Ok(())
    }
}
