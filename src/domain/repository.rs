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
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
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

    /// List direct children of `path` in the archive.
    /// `""` (empty string) lists root-level items.
    /// Returns `NotFound` if the path doesn't exist.
    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError>;
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

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// A mock repository that returns control over its behavior.
    pub struct MockArchiveRepository {
        pub entries: Vec<ArchiveEntry>,
    }

    impl MockArchiveRepository {
        pub fn new(entries: Vec<ArchiveEntry>) -> Self {
            Self { entries }
        }

        pub fn with_count(n: usize) -> Self {
            let entries: Vec<ArchiveEntry> = (0..n).map(|i| ArchiveEntry {
                name: format!("file_{}.txt", i),
                path: format!("file_{}.txt", i),
                size: 100,
                compressed_size: 50,
                is_directory: false,
                is_encrypted: false,
                is_symlink: false,
                modified: None,
                crc: None,
                original_index: i as u32,
            }).collect();
            Self { entries }
        }

        pub fn arc_with_count(n: usize) -> Arc<dyn ArchiveRepository> {
            Arc::new(Self::with_count(n))
        }
    }

    impl ArchiveRepository for MockArchiveRepository {
        fn open(&self, path: &Path, _password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
            // Return a dummy handle
            Ok(ArchiveHandle::new_reader(std::ptr::null_mut()))
        }

        fn create(&self, _path: &Path, _format: ArchiveFormat, _encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
            Err(ArchiveError::UnsupportedOperation)
        }

        fn list_page(&self, _archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError> {
            let items: Vec<ArchiveEntry> = self.entries.iter().skip(offset).take(limit).cloned().collect();
            Ok(Page::new(items, offset, Some(self.entries.len())))
        }

        fn get_properties(&self, _archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
            Ok(ArchiveProperties {
                items_count: self.entries.len() as u32,
                folders_count: 0,
                files_count: self.entries.len() as u32,
                total_size: self.entries.iter().map(|e| e.size).sum(),
                packed_size: self.entries.iter().map(|e| e.compressed_size).sum(),
                is_encrypted: false,
                has_encrypted_items: false,
                is_multi_volume: false,
                is_solid: false,
            })
        }

        fn extract(&self, _archive: &ArchiveHandle, _indices: &[u32], _dest: &Path) -> Result<(), ArchiveError> {
            Ok(())
        }

        fn extract_to_buffer(&self, _archive: &ArchiveHandle, _index: u32) -> Result<Vec<u8>, ArchiveError> {
            Err(ArchiveError::UnsupportedOperation)
        }

        fn add(&self, _archive: &mut ArchiveHandle, _files: &[PathBuf]) -> Result<(), ArchiveError> {
            Err(ArchiveError::UnsupportedOperation)
        }

        fn delete(&self, _archive: &mut ArchiveHandle, _indices: &[u32]) -> Result<(), ArchiveError> {
            Err(ArchiveError::UnsupportedOperation)
        }

        fn rename(&self, _archive: &mut ArchiveHandle, _index: u32, _new_name: &str) -> Result<(), ArchiveError> {
            Err(ArchiveError::UnsupportedOperation)
        }

        fn test(&self, _archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
            Ok(TestResult { total: 0, passed: 0, failures: vec![] })
        }

        fn list_directory(&self, _archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
            let prefix = if path.is_empty() { String::new() } else { path.to_string() };
            let plen = prefix.len();
            let result: Vec<ArchiveEntry> = self.entries.iter()
                .filter(|e| {
                    if plen == 0 { return e.path.find('/').is_none(); }
                    e.path.starts_with(&prefix) && e.path[plen..].find('/').is_none()
                })
                .cloned()
                .collect();
            Ok(result)
        }

        fn close(&self, _archive: ArchiveHandle) {}
    }
}
