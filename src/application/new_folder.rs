use crate::domain::archive::{ArchiveHandle, Password};
use crate::domain::repository::*;
use std::sync::Arc;

/// Creates a new empty directory inside an archive by adding a zero-byte
/// placeholder file at `{folder_path}/.bit7z_keep`.  bit7z implicitly creates
/// the directory tree when the placeholder is written.  The placeholder file
/// is intentionally left in the archive as a marker — it is harmless and
/// occupies zero bytes compressed.
pub fn new_folder(
    repo: Arc<dyn ArchiveRepository>,
    archive: &mut ArchiveHandle,
    folder_path: &str,
    password: Option<&Password>,
) -> Result<(), ArchiveError> {
    let folder_path = folder_path.trim_end_matches('/').trim_end_matches('\\');
    if folder_path.is_empty() {
        return Err(ArchiveError::Internal("folder_path must not be empty".into()));
    }

    let mut temp = std::env::temp_dir();
    let placeholder = ".bit7z_keep";
    temp.push(placeholder);
    std::fs::write(&temp, b"").map_err(ArchiveError::Io)?;

    let archive_inner_path = format!("{}/{}", folder_path, placeholder);
    repo.add_file_to_path(archive, &temp, &archive_inner_path, password)
}
