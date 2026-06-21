use crate::domain::archive::ArchiveHandle;

/// UI events emitted by ViewModels and consumed by Views.
#[derive(Debug, Clone)]
pub enum ArchiveVmEvent {
    SelectionChanged(Option<(ArchiveHandle, u32)>),
    RequestShowExtract,
    RequestShowCreate,
    RequestShowAdd,
    RequestShowSettings,
    RequestTest,
    RequestDelete,
    RequestAddFiles,
    RequestTestEntries { selected_only: bool },
    RequestRename { index: u32, new_name: String },
    RequestNewFolder,
    RequestNewFile,
    RequestOpenEntry,
    RequestViewEntry,
    RequestEditEntry,
    RequestProperties,
    RequestChecksum { algorithm: ChecksumAlgorithm },
    RequestPassword { path: String },
    RefreshListing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Crc32,
    Md5,
    Sha1,
    Sha256,
}
