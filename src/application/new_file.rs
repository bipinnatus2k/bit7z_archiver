use crate::application::add_to::AddToArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Creates a new empty temp file, opens it with the OS default editor,
/// then after the editor closes, adds the file to the archive if it was modified.
pub fn new_file_and_add(
    repo: Arc<dyn ArchiveRepository>,
    archive: &mut ArchiveHandle,
    file_name: &str,
) -> Result<(), ArchiveError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(file_name);

    // Create an empty temp file
    std::fs::write(&temp_path, b"").map_err(ArchiveError::Io)?;

    // Record initial metadata to detect modifications
    let initial_meta = std::fs::metadata(&temp_path).map_err(ArchiveError::Io)?;
    let initial_size = initial_meta.len();
    let initial_modified = initial_meta
        .modified()
        .map_err(ArchiveError::Io)?;

    // Open with editor — blocks until the editor exits
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "start", "/wait", "", temp_path.to_str().unwrap_or("")])
            .status()
            .map_err(|e| ArchiveError::Internal(format!("Failed to launch editor: {}", e)))?;
        if !status.success() {
            return Err(ArchiveError::Internal("Editor exited with non-zero status".into()));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let status = std::process::Command::new(&editor)
            .arg(temp_path.to_str().unwrap_or(""))
            .status()
            .map_err(|e| ArchiveError::Internal(format!("Failed to launch editor '{}': {}", editor, e)))?;
        if !status.success() {
            return Err(ArchiveError::Internal(format!("Editor '{}' exited with non-zero status", editor)));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .args(["-W", temp_path.to_str().unwrap_or("")])
            .status()
            .map_err(|e| ArchiveError::Internal(format!("Failed to launch editor: {}", e)))?;
        if !status.success() {
            return Err(ArchiveError::Internal("Editor exited with non-zero status".into()));
        }
    }

    // Check if file was modified
    let after_meta = std::fs::metadata(&temp_path).map_err(ArchiveError::Io)?;
    let after_size = after_meta.len();
    let after_modified = after_meta
        .modified()
        .map_err(ArchiveError::Io)?;

    let was_modified = after_size != initial_size || after_modified != initial_modified;
    if was_modified {
        let uc = AddToArchiveUseCase::new(repo);
        let files = [temp_path];
        uc.execute(archive, &files, None)?;
    }

    Ok(())
}
