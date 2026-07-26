use bit7z_domain::archive::{ArchiveHandle, OverwriteMode};
use std::path::PathBuf;

pub struct ExtractArchive {
    pub handle: ArchiveHandle,
    pub entry_indices: Vec<u32>,
    pub destination: PathBuf,
    pub overwrite_mode: OverwriteMode,
}
