use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use bit7z_domain::repository::NoopNotifier;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct OpenEntryUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl OpenEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
    ) -> Result<(), ArchiveError> {
        let page = self.repo.list_page(archive, index as usize, 1)?;
        if page.items.is_empty() {
            return Err(ArchiveError::NotFound(format!("index {}", index)));
        }
        let entry = &page.items[0];

        if entry.is_directory {
            return Err(ArchiveError::Internal("Cannot open a directory entry".into()));
        }

        let mut temp_path = std::env::temp_dir();
        temp_path.push(&entry.name);

        let indices = [index];
        let options = ExtractOptions {
            overwrite_mode: OverwriteMode::Overwrite,
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            notifier: Arc::new(NoopNotifier),
        };
        self.repo.extract(archive, &indices, &temp_path, &options)?;

        #[cfg(target_os = "windows")]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "start", "", temp_path.to_str().unwrap_or("")])
                .spawn()
                .map_err(|e| ArchiveError::Internal(format!("Failed to launch: {}", e)))?;
            let _ = status;
        }

        #[cfg(target_os = "linux")]
        {
            let status = std::process::Command::new("xdg-open")
                .arg(temp_path.to_str().unwrap_or(""))
                .spawn()
                .map_err(|e| ArchiveError::Internal(format!("Failed to launch: {}", e)))?;
            let _ = status;
        }

        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("open")
                .arg(temp_path.to_str().unwrap_or(""))
                .spawn()
                .map_err(|e| ArchiveError::Internal(format!("Failed to launch: {}", e)))?;
            let _ = status;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_open_entry_directory_returns_error() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "mydir".into(), path: "mydir".into(), is_directory: true, original_index: 0, ..Default::default() },
        ]);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = OpenEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0);
        assert!(matches!(result, Err(ArchiveError::Internal(ref msg)) if msg.contains("directory")));
    }

    #[test]
    fn test_open_entry_not_found() {
        let repo = MockArchiveRepository::arc_with_count(3);
        let uc = OpenEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 99);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
