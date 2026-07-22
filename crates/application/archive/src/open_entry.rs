use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::{ArchiveHandle, OverwriteMode};
use bit7z_domain::repository::{ArchiveError, ExtractOptions, NoopNotifier};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct OpenEntryUseCase {
    service: Arc<ArchiveService>,
}

impl OpenEntryUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(&self, archive: &ArchiveHandle, index: u32) -> Result<(), ArchiveError> {
        let page = self.service.list_page(archive, index as usize, 1)?;
        if page.items.is_empty() {
            return Err(ArchiveError::NotFound(format!("index {}", index)));
        }
        let entry = &page.items[0];

        if entry.is_directory {
            return Err(ArchiveError::Internal(
                "Cannot open a directory entry".into(),
            ));
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
        self.service
            .extract(archive, &indices, &temp_path, &options)?;

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
