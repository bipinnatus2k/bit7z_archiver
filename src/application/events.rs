use crate::domain::archive::ArchiveHandle;

/// Application-layer events emitted by ViewModels and consumed by Views.
/// Defined here to avoid adapter->adapter dependency.
pub enum ArchiveVmEvent {
    SelectionChanged(Option<(ArchiveHandle, u32)>),
    RequestShowExtract,
    RequestShowCreate,
    RequestTest,
}
