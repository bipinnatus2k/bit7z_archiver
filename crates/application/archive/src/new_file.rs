use crate::use_case::add_to::AddToArchiveUseCase;
use crate::runtime_service::ArchiveService;
use bit7z_domain::archive::{ArchiveHandle, Password};
use bit7z_domain::archive::progress::ArchiveError;
use std::sync::Arc;

pub fn new_file_and_add(
    service: Arc<ArchiveService>,
    archive: &ArchiveHandle,
    file_name: &str,
    _password: Option<&Password>,
) -> Result<(), ArchiveError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(file_name);

    std::fs::write(&temp_path, b"").map_err(ArchiveError::Io)?;

    let initial_meta = std::fs::metadata(&temp_path).map_err(ArchiveError::Io)?;
    let initial_size = initial_meta.len();
    let initial_modified = initial_meta.modified().map_err(ArchiveError::Io)?;

    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "start", "/wait", "", temp_path.to_str().unwrap_or("")])
            .status()
            .map_err(|e| ArchiveError::Internal(format!("Failed to launch editor: {}", e)))?;
        if !status.success() {
            return Err(ArchiveError::Internal(
                "Editor exited with non-zero status".into(),
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let status = std::process::Command::new(&editor)
            .arg(temp_path.to_str().unwrap_or(""))
            .status()
            .map_err(|e| {
                ArchiveError::Internal(format!("Failed to launch editor '{}': {}", editor, e))
            })?;
        if !status.success() {
            return Err(ArchiveError::Internal(format!(
                "Editor '{}' exited with non-zero status",
                editor
            )));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .args(["-W", temp_path.to_str().unwrap_or("")])
            .status()
            .map_err(|e| ArchiveError::Internal(format!("Failed to launch editor: {}", e)))?;
        if !status.success() {
            return Err(ArchiveError::Internal(
                "Editor exited with non-zero status".into(),
            ));
        }
    }

    let after_meta = std::fs::metadata(&temp_path).map_err(ArchiveError::Io)?;
    let after_size = after_meta.len();
    let after_modified = after_meta.modified().map_err(ArchiveError::Io)?;

    let was_modified = after_size != initial_size || after_modified != initial_modified;
    if was_modified {
        let uc = AddToArchiveUseCase::new(service);
        let files = [temp_path];
        uc.execute(archive, &files, None)?;
    }

    Ok(())
}
