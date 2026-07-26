use bit7z_domain::archive::{ArchiveFormat, EncryptionConfig};
use std::path::PathBuf;

pub struct CreateArchive {
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub encryption: Option<EncryptionConfig>,
}
