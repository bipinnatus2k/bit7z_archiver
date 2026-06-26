use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct OpenEntryUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl OpenEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    /// Temp-extract a single non-directory entry and open it with the OS default handler.
    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
    ) -> Result<(), ArchiveError> {
        // Get entry info
        let page = self.repo.list_page(archive, index as usize, 1)?;
        if page.items.is_empty() {
            return Err(ArchiveError::NotFound(format!("index {}", index)));
        }
        let entry = &page.items[0];

        if entry.is_directory {
            return Err(ArchiveError::Internal("Cannot open a directory entry".into()));
        }

        // Extract to temp file
        let mut temp_path = std::env::temp_dir();
        temp_path.push(&entry.name);

        let indices = [index];
        self.repo.extract(archive, &indices, &temp_path)?;

        // Open with OS association
        #[cfg(target_os = "windows")]
        {
            // Use `cmd /c start "" <path>` to open with default handler
            let status = std::process::Command::new("cmd")
                .args(["/c", "start", "", temp_path.to_str().unwrap_or("")])
                .spawn()
                .map_err(|e| ArchiveError::Internal(format!("Failed to launch: {}", e)))?;
            // Don't wait — let the user interact with the file
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
    use crate::domain::repository::test_utils::MockArchiveRepository;
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
