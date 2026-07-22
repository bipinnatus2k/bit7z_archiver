use bit7z_app_checksum::ChecksumAlgorithm;
use bit7z_domain::archive::ArchiveHandle;

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
