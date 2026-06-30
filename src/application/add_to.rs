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
        archive: &ArchiveHandle,
        files: &[PathBuf],
        progress: Option<ProgressSender>,
    ) -> Result<(), ArchiveError> {
        self.execute_with_password(archive, files, progress, None)
    }

    pub fn execute_with_password(
        &self,
        archive: &ArchiveHandle,
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

    #[test]
    fn test_add_to_archive_with_progress() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let test_file = std::env::temp_dir().join("add_to_progress_test.txt");
        std::fs::write(&test_file, b"progress").unwrap();
        let (tx, _rx) = crate::application::progress::progress_channel();
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[test_file.clone()], Some(tx));
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_add_to_archive_with_password() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let test_file = std::env::temp_dir().join("add_to_password_test.txt");
        std::fs::write(&test_file, b"pw test").unwrap();
        let handle = ArchiveHandle::new_writer().with_path("secret.7z".into());
        let pw = Password::new("hunter2");
        let result = uc.execute_with_password(&handle, &[test_file.clone()], None, Some(&pw));
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_add_to_archive_directory_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let uc = AddToArchiveUseCase::new(repo);
        let test_dir = std::env::temp_dir().join("add_to_dir_test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let handle = ArchiveHandle::new_writer().with_path("test.7z".into());
        let result = uc.execute(&handle, &[test_dir.clone()], None);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir(&test_dir);
    }
}
