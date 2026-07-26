use bit7z_domain::archive::{ArchiveFormat, EncryptionConfig};
use std::path::PathBuf;

/// Create a new archive.
/// TODO(Phase 2): Add `pub files: Vec<FileToAdd>` field.
pub struct CreateArchive {
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub encryption: Option<EncryptionConfig>,
}
