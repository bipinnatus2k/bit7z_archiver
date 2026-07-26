use bit7z_domain::archive::ArchiveHandle;
use std::path::PathBuf;

/// Add files to an existing archive.
/// TODO(Phase 2): Add `pub password: Option<Password>` and `pub encrypt_filenames: bool` fields.
pub struct AddFilesToArchive {
    pub handle: ArchiveHandle,
    pub files: Vec<PathBuf>,
}
