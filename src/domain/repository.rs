use crate::domain::archive::*;
use gpui::Global;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Global wrapper for repository access.
#[derive(Clone)]
pub struct RepoGlobal(pub Arc<dyn ArchiveRepository>);
impl Global for RepoGlobal {}

/// Core repository trait for archive operations.
/// Implementations wrap the bit7z C++ bridge.
pub trait ArchiveRepository: Send + Sync {
    fn open(&self, path: &Path, password: Option<&str>) -> Result<ArchiveHandle, ArchiveError>;
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError>;
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path) -> Result<(), ArchiveError>;
    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn add(&self, archive: &mut ArchiveHandle, files: &[PathBuf]) -> Result<(), ArchiveError>;
    fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32]) -> Result<(), ArchiveError>;
    fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError>;
    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError>;
    fn close(&self, archive: ArchiveHandle);
}

/// Domain-level errors for archive operations.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Unsupported archive format")]
    UnsupportedFormat,
    #[error("Archive is corrupt: {0}")]
    Corrupt(String),
    #[error("Wrong password")]
    WrongPassword,
    #[error("Archive is encrypted - password required")]
    EncryptedArchiveRequiresPassword,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Operation canceled")]
    Canceled,
    #[error("Operation not supported for this archive format")]
    UnsupportedOperation,
    #[error("Archive is not writable")]
    ReadOnlyArchive,
}

/// Archive-level properties from bit7z.
#[derive(Debug, Clone)]
pub struct ArchiveProperties {
    pub items_count: u32,
    pub folders_count: u32,
    pub files_count: u32,
    pub total_size: u64,
    pub packed_size: u64,
    pub is_encrypted: bool,
    pub has_encrypted_items: bool,
    pub is_multi_volume: bool,
    pub is_solid: bool,
}
