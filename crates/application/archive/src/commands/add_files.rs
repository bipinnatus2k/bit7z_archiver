use bit7z_domain::archive::ArchiveHandle;
use std::path::PathBuf;

pub struct AddFilesToArchive {
    pub handle: ArchiveHandle,
    pub files: Vec<PathBuf>,
}
