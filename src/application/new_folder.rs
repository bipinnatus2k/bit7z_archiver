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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_new_folder_success() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let mut handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &mut handle, "newdir", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_folder_empty_path() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let mut handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &mut handle, "", None);
        assert!(matches!(result, Err(ArchiveError::Internal(ref msg)) if msg.contains("empty")));
    }

    #[test]
    fn test_new_folder_nested() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let mut handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &mut handle, "parent/child", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_folder_trim_trailing_slash() {
        let repo = MockArchiveRepository::arc_with_count(0);
        let mut handle = ArchiveHandle::new_reader();
        let result = new_folder(repo, &mut handle, "mydir/", None);
        assert!(result.is_ok());
    }
}
