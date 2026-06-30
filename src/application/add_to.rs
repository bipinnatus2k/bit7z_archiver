use crate::application::modify::ModifyArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AddToArchiveUseCase {
    inner: ModifyArchiveUseCase,
}

impl AddToArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { inner: ModifyArchiveUseCase::new(repo) }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        files: &[PathBuf],
        progress: Option<Arc<dyn ProgressNotifier>>,
    ) -> Result<(), ArchiveError> {
        self.inner.add_files(archive, files, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_add_to_archive_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let test_file = std::env::temp_dir().join("add_to_test_file.txt");
        std::fs::write(&test_file, b"test").unwrap();
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[test_file.clone()], None);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_add_to_archive_file_not_found() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[PathBuf::from(r"Z:\nonexistent\file.txt")], None);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}