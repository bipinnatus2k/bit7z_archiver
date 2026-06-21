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

pub struct RootController {
    repo: Arc<dyn ArchiveRepository>,
}

impl RootController {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
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
    ) -> Result<(), ArchiveError> {
        let uc = ExtractEntriesUseCase::new(self.repo.clone());
        uc.execute(archive, indices, dest)
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
        self.repo.close(archive);
    }
}


