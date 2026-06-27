use crate::domain::archive::*;
use std::path::{Path, PathBuf};

/// Progress update message sent during long-running operations.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub file_current: u64,
    pub file_total: u64,
    pub current_file: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

/// Abstraction for progress notification (domain port, adapter implements).
pub trait ProgressNotifier: Send + Sync {
    fn notify(&self, update: &ProgressUpdate);
}

/// Core repository trait for archive operations.
/// Implementations wrap the bit7z C++ bridge.
pub trait ArchiveRepository: Send + Sync {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError>;
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path) -> Result<(), ArchiveError>;
    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn add(&self, archive: &mut ArchiveHandle, files: &[PathBuf], password: Option<&Password>) -> Result<(), ArchiveError>;
    fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32]) -> Result<(), ArchiveError>;
    fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError>;
    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError>;
    fn close(&self, archive: ArchiveHandle);

    /// Set a progress notifier for long-running operations.
    fn set_progress_notifier(&self, _notifier: Box<dyn ProgressNotifier>) {}

    /// List direct children of `path` in the archive.
    /// `""` (empty string) lists root-level items.
    /// Returns `NotFound` if the path doesn't exist.
    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError>;

    /// Add a single file to the archive at a specific archive-internal path.
    fn add_file_to_path(&self, _archive: &mut ArchiveHandle, _file_path: &Path, _archive_path: &str, _password: Option<&Password>) -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }
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
    pub encrypted_names: bool,
    pub has_comment: bool,
    pub comment_size: Option<usize>,
    pub has_recovery_record: bool,
    pub locked: bool,
    pub dictionary_size: Option<u64>,
    pub headers_size: u64,
    pub volumes_count: u32,
}

impl Default for ArchiveProperties {
    fn default() -> Self {
        Self {
            items_count: 0,
            folders_count: 0,
            files_count: 0,
            total_size: 0,
            packed_size: 0,
            is_encrypted: false,
            has_encrypted_items: false,
            is_multi_volume: false,
            is_solid: false,
            encrypted_names: false,
            has_comment: false,
            comment_size: None,
            has_recovery_record: false,
            locked: false,
            dictionary_size: None,
            headers_size: 0,
            volumes_count: 0,
        }
    }
}

#[doc(hidden)]
pub mod test_utils {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// A mock repository that returns control over its behavior.
    pub struct MockArchiveRepository {
        pub entries: Mutex<Vec<ArchiveEntry>>,
        /// Preset test result to return on `test()`.
        pub test_result: Mutex<TestResult>,
        /// If true, `extract_to_buffer` actually returns data using the CRC as content.
        pub mock_extract_buffer: bool,
        crate_handle: Mutex<Option<ArchiveHandle>>,
    }

    impl MockArchiveRepository {
        pub fn new(entries: Vec<ArchiveEntry>) -> Self {
            Self {
                entries: Mutex::new(entries),
                test_result: Mutex::new(TestResult { total: 0, passed: 0, failed: vec![] }),
                mock_extract_buffer: false,
                crate_handle: Mutex::new(None),
            }
        }

        pub fn with_count(n: usize) -> Self {
            let entries: Vec<ArchiveEntry> = (0..n).map(|i| ArchiveEntry {
                name: format!("file_{}.txt", i),
                path: format!("file_{}.txt", i),
                size: 100,
                compressed_size: 50,
                crc: Some(i as u32),
                original_index: i as u32,
                ..Default::default()
            }).collect();
            Self {
                entries: Mutex::new(entries),
                test_result: Mutex::new(TestResult { total: n, passed: n, failed: vec![] }),
                mock_extract_buffer: false,
                crate_handle: Mutex::new(None),
            }
        }

        pub fn arc_with_count(n: usize) -> Arc<dyn ArchiveRepository> {
            Arc::new(Self::with_count(n))
        }

        pub fn with_test_result(self, result: TestResult) -> Self {
            *self.test_result.lock().unwrap() = result;
            self
        }

        pub fn with_extract_buffer(mut self, enabled: bool) -> Self {
            self.mock_extract_buffer = enabled;
            self
        }

        pub fn entry_count(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
    }

    impl ArchiveRepository for MockArchiveRepository {
        fn open(&self, _path: &Path, _password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
            let handle = ArchiveHandle::new_reader();
            *self.crate_handle.lock().unwrap() = Some(handle.clone());
            Ok(handle)
        }

        fn create(&self, path: &Path, _format: ArchiveFormat, _encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
            let handle = ArchiveHandle::new_writer()
                .with_path(path.to_path_buf());
            *self.crate_handle.lock().unwrap() = Some(handle.clone());
            Ok(handle)
        }

        fn list_page(&self, _archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let items: Vec<ArchiveEntry> = entries.iter().skip(offset).take(limit).cloned().collect();
            Ok(Page::new(items, offset, Some(entries.len())))
        }

        fn get_properties(&self, _archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let files = entries.iter().filter(|e| !e.is_directory).count() as u32;
            let folders = entries.iter().filter(|e| e.is_directory).count() as u32;
            Ok(ArchiveProperties {
                items_count: entries.len() as u32,
                folders_count: folders,
                files_count: files,
                total_size: entries.iter().map(|e| e.size).sum(),
                packed_size: entries.iter().map(|e| e.compressed_size).sum(),
                ..Default::default()
            })
        }

        fn extract(&self, _archive: &ArchiveHandle, _indices: &[u32], _dest: &Path) -> Result<(), ArchiveError> {
            Ok(())
        }

        fn extract_to_buffer(&self, _archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError> {
            if !self.mock_extract_buffer {
                return Err(ArchiveError::UnsupportedOperation);
            }
            let entries = self.entries.lock().unwrap();
            let entry = entries.iter().find(|e| e.original_index == index)
                .ok_or_else(|| ArchiveError::NotFound(format!("index {}", index)))?;
            // Return deterministic data: size bytes filled with index as u8
            // This way tests can pre-compute expected CRCs
            let size = entry.size as usize;
            let fill = (index as u8).wrapping_mul(17);
            Ok(vec![fill; size.max(1)])
        }

        fn add(&self, _archive: &mut ArchiveHandle, files: &[PathBuf], _password: Option<&Password>) -> Result<(), ArchiveError> {
            let mut entries = self.entries.lock().unwrap();
            let next_idx = entries.len() as u32;
            for (i, path) in files.iter().enumerate() {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("file_{}", i));
                let path_str = name.clone();
                entries.push(ArchiveEntry {
                    name,
                    path: path_str,
                    size: 100,
                    compressed_size: 50,
                    crc: Some(next_idx + i as u32),
                    original_index: next_idx + i as u32,
                    ..Default::default()
                });
            }
            Ok(())
        }

        fn add_file_to_path(&self, _archive: &mut ArchiveHandle, file_path: &Path, archive_path: &str, _password: Option<&Password>) -> Result<(), ArchiveError> {
            if file_path.exists() {
                let name = file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".into());
                let size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
                let mut entries_lock = self.entries.lock().unwrap();
                let next_idx = entries_lock.len() as u32;
                entries_lock.push(ArchiveEntry {
                    name: name.clone(),
                    path: archive_path.to_string(),
                    size,
                    compressed_size: size / 2,
                    crc: Some(next_idx),
                    original_index: next_idx,
                    ..Default::default()
                });
                Ok(())
            } else {
                Err(ArchiveError::NotFound(file_path.to_string_lossy().to_string()))
            }
        }

        fn delete(&self, _archive: &mut ArchiveHandle, indices: &[u32]) -> Result<(), ArchiveError> {
            let mut entries = self.entries.lock().unwrap();
            let mut sorted: Vec<u32> = indices.to_vec();
            sorted.sort_unstable_by(|a, b| b.cmp(a)); // descending
            for idx in sorted {
                if let Some(pos) = entries.iter().position(|e| e.original_index == idx) {
                    entries.remove(pos);
                }
            }
            // Re-index
            for (i, e) in entries.iter_mut().enumerate() {
                e.original_index = i as u32;
            }
            Ok(())
        }

        fn rename(&self, _archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError> {
            let mut entries = self.entries.lock().unwrap();
            let entry = entries.iter_mut()
                .find(|e| e.original_index == index)
                .ok_or_else(|| ArchiveError::NotFound(format!("index {}", index)))?;
            // Update path to match new name
            let new_path = if let Some(slash_pos) = entry.path.rfind('/') {
                format!("{}/{}", &entry.path[..slash_pos], new_name)
            } else {
                new_name.to_string()
            };
            entry.name = new_name.to_string();
            entry.path = new_path;
            Ok(())
        }

        fn test(&self, _archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
            Ok(self.test_result.lock().unwrap().clone())
        }

        fn list_directory(&self, _archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let prefix = if path.is_empty() { String::new() } else { path.to_string() };
            let plen = prefix.len();
            let result: Vec<ArchiveEntry> = entries.iter()
                .filter(|e| {
                    if plen == 0 { return !e.path.contains('/'); }
                    e.path.starts_with(&prefix) && e.path[plen..].find('/').is_none()
                })
                .cloned()
                .collect();
            Ok(result)
        }

        fn close(&self, _archive: ArchiveHandle) {
            *self.crate_handle.lock().unwrap() = None;
        }
    }
}

#[cfg(test)]
mod archive_properties_tests {
    use super::*;

    #[test]
    fn test_default_properties_are_zero() {
        let props = ArchiveProperties::default();
        assert_eq!(props.items_count, 0);
        assert_eq!(props.folders_count, 0);
        assert_eq!(props.files_count, 0);
        assert_eq!(props.total_size, 0);
        assert_eq!(props.packed_size, 0);
        assert!(!props.is_encrypted);
        assert!(!props.has_encrypted_items);
        assert!(!props.is_multi_volume);
        assert!(!props.is_solid);
        assert!(!props.encrypted_names);
        assert!(!props.has_comment);
        assert_eq!(props.comment_size, None);
        assert!(!props.has_recovery_record);
        assert!(!props.locked);
        assert_eq!(props.dictionary_size, None);
    }

    #[test]
    fn test_custom_properties() {
        let props = ArchiveProperties {
            items_count: 42, folders_count: 5, files_count: 37,
            total_size: 10240, packed_size: 5120,
            is_encrypted: true, has_encrypted_items: true,
            is_multi_volume: false, is_solid: true,
            encrypted_names: true, has_comment: true,
            comment_size: Some(128), has_recovery_record: false,
            locked: true, dictionary_size: Some(65536),
            headers_size: 1200,
            volumes_count: 1,
        };
        assert_eq!(props.items_count, 42);
        assert!(props.is_encrypted);
        assert!(props.locked);
        assert_eq!(props.dictionary_size, Some(65536));
    }

    #[test]
    fn test_files_plus_folders_equals_items() {
        let props = ArchiveProperties {
            items_count: 100, folders_count: 10, files_count: 90,
            ..Default::default()
        };
        assert_eq!(props.files_count + props.folders_count, props.items_count);
    }
}
